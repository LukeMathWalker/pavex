mod cloning;
mod thread_safety;

use bimap::BiHashMap;
pub(crate) use cloning::runtime_singletons_can_be_cloned_if_needed;
use convert_case::Casing;
use guppy::PackageId;
use itertools::Itertools;
use quote::format_ident;
pub(crate) use thread_safety::runtime_singletons_are_thread_safe;

use super::{
    call_graph::RawCallGraphExt,
    components::{ComponentDb, ComponentId},
    computations::ComputationDb,
    constructibles::ConstructibleDb,
    framework_items::FrameworkItemDb,
    processing_pipeline::RequestHandlerPipeline,
};
use crate::{
    compiler::app::GENERATED_APP_PACKAGE_ID,
    language::{Callable, GenericArgument, InvocationStyle, ResolvedType},
    rustdoc::CrateCollection,
};
use indexmap::{IndexMap, IndexSet};
use pavex_bp_schema::Lifecycle;
use std::{collections::BTreeMap, ops::Deref};

/// The set of singletons that are needed to serve user requests.
///
/// A singleton won't be included in this set if it's only needed
/// to build the application state, before the application starts
/// serving requests.
pub struct ApplicationState {
    #[allow(unused)]
    type2id: IndexSet<(ResolvedType, ComponentId)>,
    bindings: BiHashMap<syn::Ident, ResolvedType>,
}

impl ApplicationState {
    /// Examine the processing pipeline of all request handlers to
    /// determine which singletons are needed to serve user requests.
    pub fn new(
        handler_id2pipeline: &IndexMap<ComponentId, RequestHandlerPipeline>,
        framework_item_db: &FrameworkItemDb,
        constructibles_db: &ConstructibleDb,
        component_db: &ComponentDb,
        computation_db: &ComputationDb,
        krate_collection: &CrateCollection,
        diagnostics: &mut crate::diagnostic::DiagnosticSink,
    ) -> Self {
        let type2id = extract_runtime_singletons(
            handler_id2pipeline.values(),
            framework_item_db,
            constructibles_db,
            component_db,
        );
        runtime_singletons_are_thread_safe(
            &type2id,
            component_db,
            computation_db,
            krate_collection,
            diagnostics,
        );
        runtime_singletons_can_be_cloned_if_needed(
            handler_id2pipeline.values(),
            component_db,
            computation_db,
            krate_collection,
            diagnostics,
        );
        let bindings = Self::assign_field_names(&type2id);
        Self { type2id, bindings }
    }

    /// Each singleton will become a field in the code-generated application state.
    /// We need to assign a unique field name to each of them.
    ///
    /// The name must be:
    /// - Unique
    /// - A valid Rust identifier
    /// - Stable, i.e. it shouldn't change between `pavexc` runs
    /// - Meaningful, i.e. it should be easy to understand what the singleton is
    ///
    /// We try to achieve this by:
    /// - Using the singleton's last path segment as the field name
    /// - If there are multiple singletons with the same last path segment, we append
    ///   the last path segment of the singleton's generic arguments to the field name,
    ///   separated by `_`, if there are any
    /// - If that's not enough, we prepend the singleton's crate name to the field name with a `_` separator
    /// - If there are still conflicts, we use the singleton's full path as the field name, replacing
    ///   all `::` with `_`
    fn assign_field_names(
        type2id: &IndexSet<(ResolvedType, ComponentId)>,
    ) -> BiHashMap<syn::Ident, ResolvedType> {
        let mut name_map = BiHashMap::new();

        let mut candidate2positions: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (i, (resolved_type, _)) in type2id.iter().enumerate() {
            candidate2positions
                .entry(field_name_candidate(
                    resolved_type,
                    NamingStrategy::LastSegment,
                ))
                .or_default()
                .push(i);
        }

        let fallback_strategies = [
            NamingStrategy::LastSegmentWithGenericArgs,
            NamingStrategy::WithCrateName,
            NamingStrategy::FullPath,
        ];
        'outer: for fallback_strategy in &fallback_strategies {
            let ambiguous_entries: Vec<_> = candidate2positions
                .iter()
                .filter_map(|(k, v)| {
                    if v.len() > 1 {
                        Some((k.clone(), v.clone()))
                    } else {
                        None
                    }
                })
                .collect();
            if ambiguous_entries.is_empty() {
                break 'outer;
            }
            for (name, positions) in ambiguous_entries {
                candidate2positions.remove(&name);
                for i in positions {
                    let ty_ = &type2id.get_index(i).unwrap().0;
                    candidate2positions
                        .entry(field_name_candidate(ty_, *fallback_strategy))
                        .or_default()
                        .push(i);
                }
            }
        }

        let still_ambiguous: Vec<usize> = candidate2positions
            .values()
            .filter(|v| v.len() > 1)
            .flatten()
            .cloned()
            .collect();
        if !still_ambiguous.is_empty() {
            panic!(
                "Failed to assign unique fields names to the singletons stored inside `ApplicationState`. \
                I couldn't disambiguate the following types:\n{}",
                still_ambiguous
                    .into_iter()
                    .map(|i| type2id.get_index(i).unwrap().0.display_for_error())
                    .join("\n-")
            );
        }

        for (name, positions) in candidate2positions {
            let position = positions[0];
            let ty_ = type2id.get_index(position).unwrap().0.to_owned();
            // The field name may be a reserved keyword.
            // If that's the case, we append a `_` to the field name.
            let name =
                syn::parse_str::<syn::Ident>(&name).unwrap_or_else(|_| format_ident!("{}_", name));
            name_map.insert(name, ty_);
        }

        name_map
    }

    /// Return the type of the application state, assuming it'll belong
    /// to the generated crate.
    pub fn type_(&self) -> crate::language::PathType {
        crate::language::PathType {
            package_id: PackageId::new(GENERATED_APP_PACKAGE_ID),
            rustdoc_id: None,
            base_type: vec!["crate".into(), "ApplicationState".into()],
            generic_arguments: vec![],
        }
    }

    /// Return a callable that can be used to build the application state, by
    /// assigning each field to an instance of the expected type.
    pub fn initializer(&self) -> Callable {
        // We build a "mock" callable that has the right inputs in order to drive the machinery
        // that builds the dependency graph.
        let ty_ = self.type_();
        let callable = Callable {
            is_async: false,
            takes_self_as_ref: false,
            path: ty_.resolved_path(),
            output: Some(ty_.into()),
            inputs: {
                // Ensure that the inputs are sorted by name.
                let b = self.bindings.iter().collect::<BTreeMap<_, _>>();
                b.into_values().cloned().collect()
            },
            invocation_style: InvocationStyle::StructLiteral {
                field_names: self
                    .bindings
                    .iter()
                    .map(|(ident, type_)| (ident.to_string(), type_.to_owned()))
                    .collect(),
                extra_field2default_value: Default::default(),
            },
            source_coordinates: None,
        };
        callable
    }

    /// Return a bi-directional map between field names and their types.
    pub fn bindings(&self) -> &BiHashMap<syn::Ident, ResolvedType> {
        &self.bindings
    }
}

#[derive(Clone, Copy)]
enum NamingStrategy {
    LastSegment,
    LastSegmentWithGenericArgs,
    WithCrateName,
    FullPath,
}

fn field_name_candidate(ty_: &ResolvedType, strategy: NamingStrategy) -> String {
    let mut candidate = String::new();
    _field_name_candidate(ty_, strategy, &mut candidate);
    candidate
}

fn _field_name_candidate(ty_: &ResolvedType, strategy: NamingStrategy, candidate: &mut String) {
    match ty_ {
        ResolvedType::ResolvedPath(path_type) => match strategy {
            NamingStrategy::LastSegment => {
                let last = path_type
                    .base_type
                    .last()
                    .expect("A type path can't be empty")
                    .to_case(convert_case::Case::Snake);
                candidate.push_str(&last);
            }
            NamingStrategy::LastSegmentWithGenericArgs => {
                let last = path_type
                    .base_type
                    .last()
                    .expect("A type path can't be empty")
                    .to_case(convert_case::Case::Snake);
                candidate.push_str(&last);
                for arg in &path_type.generic_arguments {
                    let GenericArgument::TypeParameter(ty_) = arg else {
                        continue;
                    };
                    candidate.push('_');
                    _field_name_candidate(ty_, NamingStrategy::LastSegment, candidate);
                }
            }
            NamingStrategy::WithCrateName => {
                let first = path_type
                    .base_type
                    .first()
                    .expect("A type path can't be empty")
                    .to_case(convert_case::Case::Snake);
                candidate.push_str(&first);
                candidate.push('_');
                _field_name_candidate(ty_, NamingStrategy::LastSegment, candidate);
            }
            NamingStrategy::FullPath => {
                candidate.push_str(
                    &path_type
                        .base_type
                        .iter()
                        .map(|s| s.to_case(convert_case::Case::Snake))
                        .join("_"),
                );
            }
        },
        ResolvedType::Reference(type_reference) => {
            // We never have both a reference and an owned version of the same
            // type in the application state, so we can just use the owned version
            // of the type for naming purposes.
            _field_name_candidate(&type_reference.inner, strategy, candidate);
        }
        ResolvedType::Tuple(tuple) => {
            // Please don't have tuples in the application state, they're ugly.
            for (i, ty_) in tuple.elements.iter().enumerate() {
                if i > 0 {
                    candidate.push('_');
                }
                _field_name_candidate(ty_, strategy, candidate);
            }
        }
        ResolvedType::ScalarPrimitive(scalar_primitive) => {
            candidate.push_str(scalar_primitive.as_str());
            candidate.push('_');
        }
        ResolvedType::Slice(slice) => {
            // Same reasoning as for references.
            _field_name_candidate(&slice.element_type, strategy, candidate);
        }
        ResolvedType::Generic(generic) => {
            // We don't have unassigned generics in the application state, so this should never happen.
            // But, should it happen, there's really no other way to name it.
            candidate.push_str(&generic.name.to_case(convert_case::Case::Snake));
        }
    }
}

fn extract_runtime_singletons<'a>(
    handler_pipelines: impl Iterator<Item = &'a RequestHandlerPipeline>,
    framework_item_db: &FrameworkItemDb,
    constructibles_db: &ConstructibleDb,
    component_db: &ComponentDb,
) -> IndexSet<(ResolvedType, ComponentId)> {
    let mut type2id = IndexSet::new();
    for handler_pipeline in handler_pipelines {
        for graph in handler_pipeline.graph_iter() {
            let root_component_id = graph.root_component_id();
            let root_component_scope_id = component_db.scope_id(root_component_id);
            for required_input in graph.call_graph.required_input_types() {
                let required_input = if let ResolvedType::Reference(t) = &required_input {
                    if !t.lifetime.is_static() {
                        // We can't store non-'static references in the application state, so we expect
                        // to see the referenced type in there.
                        t.inner.deref()
                    } else {
                        &required_input
                    }
                } else {
                    &required_input
                };
                // If it's a framework built-in, nothing to do!
                if framework_item_db.get_id(required_input).is_some() {
                    continue;
                }
                // This can be `None` if the required input is a singleton that is used
                // by a downstream stage of the processing pipeline.
                // The singleton will be passed down using `Next` pass-along state from
                // the very first stage in the pipeline all the way to the stage that needs it,
                // but the singleton constructor might not be visible in the scope of the current
                // stage of the processing pipeline.
                if let Some((component_id, _)) = constructibles_db.get(
                    root_component_scope_id,
                    required_input,
                    component_db.scope_graph(),
                ) {
                    let lifecycle = component_db.lifecycle(component_id);
                    #[cfg(debug_assertions)]
                    {
                        // No scenario where this should/could happen.
                        assert_ne!(lifecycle, Lifecycle::Transient);
                    }

                    // Some inputs are request-scoped because they come from the `Next<_>` pass-along
                    // state. We don't care about those here.
                    if lifecycle == Lifecycle::Singleton {
                        type2id.insert((required_input.to_owned(), component_id));
                    }
                }
            }
        }
    }
    type2id
}
