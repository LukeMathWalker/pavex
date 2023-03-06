use ahash::{HashMap, HashMapExt, HashSet};
use guppy::graph::PackageGraph;
use indexmap::IndexSet;
use miette::NamedSource;
use syn::spanned::Spanned;

use pavex_builder::Lifecycle;

use crate::compiler::analyses::components::{ComponentDb, ComponentId, HydratedComponent};
use crate::compiler::analyses::computations::ComputationDb;
use crate::compiler::analyses::raw_identifiers::RawCallableIdentifiersDb;
use crate::compiler::analyses::user_components::{UserComponentDb, UserComponentId};
use crate::diagnostic;
use crate::diagnostic::{
    convert_proc_macro_span, convert_rustdoc_span, read_source_file, AnnotatedSnippet,
    CompilerDiagnostic, LocationExt, SourceSpanExt,
};
use crate::language::{Callable, ResolvedType};
use crate::rustdoc::CrateCollection;

#[derive(Debug)]
pub(crate) struct ConstructibleDb {
    type2constructor_id: HashMap<ResolvedType, ComponentId>,
    /// Every time we encounter a constructible type that contains an unassigned generic type
    /// (e.g. `T` in `Vec<T>` instead of `u8` in `Vec<u8>`), we store it here.
    ///
    /// This enables us to quickly determine if there might be a constructor for a given concrete
    /// type.
    /// For example, if you have a `Vec<u8>`, you first look in `type2constructor_id` to see if
    /// there is a constructor that returns `Vec<u8>`. If there isn't, you look in
    /// `generic_base_types` to see if there is a constructor that returns `Vec<T>`.
    ///
    /// Specialization, in a nutshell!
    templated_constructors: IndexSet<ResolvedType>,
}

impl ConstructibleDb {
    pub(crate) fn build(
        component_db: &mut ComponentDb,
        computation_db: &mut ComputationDb,
        package_graph: &PackageGraph,
        krate_collection: &CrateCollection,
        user_component_db: &UserComponentDb,
        raw_identifiers_db: &RawCallableIdentifiersDb,
        request_scoped_framework_types: &HashSet<&ResolvedType>,
        diagnostics: &mut Vec<miette::Error>,
    ) -> Self {
        let mut type2constructor_id = HashMap::new();
        let mut templated_constructors = IndexSet::new();
        for (component_id, component) in component_db.constructors(computation_db) {
            let output = component.output_type();
            type2constructor_id.insert(output.to_owned(), component_id);
            if output.is_a_template() {
                templated_constructors.insert(output.to_owned());
            }
        }
        let mut self_ = Self {
            type2constructor_id,
            templated_constructors,
        };

        let mut component_ids = component_db.iter().map(|(id, _)| id).collect::<Vec<_>>();
        let mut n_component_ids = component_ids.len();
        loop {
            for component_id in component_ids {
                let resolved_component =
                    component_db.hydrated_component(component_id, computation_db);
                // We don't support dependency injection for transformers (yet).
                if let HydratedComponent::Transformer(_) = &resolved_component {
                    continue;
                }

                if let HydratedComponent::Constructor(_) = &resolved_component {
                    let lifecycle = component_db.lifecycle(component_id).unwrap();
                    if lifecycle == &Lifecycle::Singleton {
                        continue;
                    }
                }

                let input_types = {
                    let mut input_types: Vec<Option<ResolvedType>> = resolved_component
                        .input_types()
                        .iter()
                        .map(|i| Some(i.to_owned()))
                        .collect();
                    // Errors happen, they are not "constructed" (we use a transformer instead).
                    // Therefore we skip the error input type for error handlers.
                    if let HydratedComponent::ErrorHandler(e) = &resolved_component {
                        input_types[e.error_input_index] = None;
                    }
                    input_types
                };

                'outer: for (input_index, input) in input_types.into_iter().enumerate() {
                    let input = match input.as_ref() {
                        Some(i) => i,
                        None => {
                            continue;
                        }
                    };
                    for request_scoped_framework_type in request_scoped_framework_types {
                        if request_scoped_framework_type
                            .is_a_template_for(input)
                            .is_some()
                        {
                            continue 'outer;
                        }
                    }
                    if self_.get(input).is_some() {
                        continue;
                    }
                    for templated_constructible_type in &self_.templated_constructors {
                        if let Some(bindings) =
                            templated_constructible_type.is_a_template_for(input)
                        {
                            bind_and_register_constructor(
                                self_[templated_constructible_type],
                                component_db,
                                computation_db,
                                &mut self_,
                                &bindings,
                            );
                            continue 'outer;
                        }
                    }
                    if let Some(user_component_id) = component_db.user_component_id(component_id) {
                        ConstructibleDb::missing_constructor(
                            user_component_id,
                            user_component_db,
                            input,
                            input_index,
                            package_graph,
                            krate_collection,
                            raw_identifiers_db,
                            computation_db,
                            diagnostics,
                        )
                    } else {
                        unreachable!()
                    }
                }
            }

            // If we didn't add any new component IDs, we're done.
            // Otherwise, we need to determine the list of component IDs that we are yet to examine.
            let new_component_ids: Vec<_> = component_db
                .iter()
                .skip(n_component_ids)
                .map(|(id, _)| id)
                .collect();
            if new_component_ids.is_empty() {
                break;
            } else {
                n_component_ids += new_component_ids.len();
                component_ids = new_component_ids;
            }
        }

        self_
    }

    fn missing_constructor(
        user_component_id: UserComponentId,
        user_component_db: &UserComponentDb,
        unconstructible_type: &ResolvedType,
        unconstructible_type_index: usize,
        package_graph: &PackageGraph,
        krate_collection: &CrateCollection,
        raw_identifiers_db: &RawCallableIdentifiersDb,
        computation_db: &ComputationDb,
        diagnostics: &mut Vec<miette::Error>,
    ) {
        fn get_definition_info(
            callable: &Callable,
            unconstructible_type_index: usize,
            package_graph: &PackageGraph,
            krate_collection: &CrateCollection,
        ) -> Option<AnnotatedSnippet> {
            let (callable_type, _) = callable.path.find_rustdoc_items(krate_collection).ok()?;
            let callable_item = callable_type.item.item;
            let definition_span = callable_item.span.as_ref()?;
            let source_contents =
                read_source_file(&definition_span.filename, &package_graph.workspace()).ok()?;
            let span = convert_rustdoc_span(&source_contents, definition_span.to_owned());
            let span_contents = &source_contents[span.offset()..(span.offset() + span.len())];
            let input = match &callable_item.inner {
                rustdoc_types::ItemEnum::Function(_) => {
                    if let Ok(item) = syn::parse_str::<syn::ItemFn>(span_contents) {
                        let mut inputs = item.sig.inputs.iter();
                        inputs.nth(unconstructible_type_index).cloned()
                    } else if let Ok(item) = syn::parse_str::<syn::ImplItemMethod>(span_contents) {
                        let mut inputs = item.sig.inputs.iter();
                        inputs.nth(unconstructible_type_index).cloned()
                    } else {
                        eprintln!("Could not parse as a function or method:\n{span_contents}");
                        return None;
                    }
                }
                _ => unreachable!(),
            }?;
            let s = convert_proc_macro_span(
                span_contents,
                match input {
                    syn::FnArg::Typed(typed) => typed.ty.span(),
                    syn::FnArg::Receiver(r) => r.span(),
                },
            );
            let label = miette::SourceSpan::new(
                // We must shift the offset forward because it's the
                // offset from the beginning of the file slice that
                // we deserialized, instead of the entire file
                (s.offset() + span.offset()).into(),
                s.len().into(),
            )
            .labeled("I don't know how to construct an instance of this input parameter".into());
            let source_path = definition_span.filename.to_str().unwrap();
            Some(AnnotatedSnippet::new(
                NamedSource::new(source_path, source_contents),
                label,
            ))
        }

        let user_component = &user_component_db[user_component_id];
        let callable = &computation_db[user_component_id];
        let raw_identifier_id = user_component.raw_callable_identifiers_id();
        let component_kind = user_component.callable_type();

        let location = raw_identifiers_db.get_location(raw_identifier_id);
        let source = match location.source_file(package_graph) {
            Ok(s) => s,
            Err(e) => {
                diagnostics.push(e.into());
                return;
            }
        };
        let label = diagnostic::get_f_macro_invocation_span(&source, location)
            .map(|s| s.labeled(format!("The {component_kind} was registered here")));
        let e = anyhow::anyhow!(
                "I can't invoke your {component_kind}, `{}`, because it needs an instance \
                of `{unconstructible_type:?}` as input, but I can't find a constructor for that type.",
                callable.path
            );
        let definition_info = get_definition_info(
            callable,
            unconstructible_type_index,
            package_graph,
            krate_collection,
        );
        let diagnostic = CompilerDiagnostic::builder(source, e)
            .optional_label(label)
            .optional_additional_annotated_snippet(definition_info)
            .help(format!(
                "Register a constructor for `{unconstructible_type:?}`"
            ))
            .build();
        diagnostics.push(diagnostic.into());
    }

    pub(crate) fn get(&self, t: &ResolvedType) -> Option<ComponentId> {
        self.type2constructor_id.get(t).cloned()
    }
}

impl std::ops::Index<&ResolvedType> for ConstructibleDb {
    type Output = ComponentId;

    fn index(&self, index: &ResolvedType) -> &Self::Output {
        &self.type2constructor_id[index]
    }
}

fn bind_and_register_constructor(
    templated_component_id: ComponentId,
    component_db: &mut ComponentDb,
    computation_db: &mut ComputationDb,
    constructible_db: &mut ConstructibleDb,
    bindings: &HashMap<String, ResolvedType>,
) {
    let bound_component_id =
        component_db.bind_generic_type_parameters(templated_component_id, bindings, computation_db);
    let mut derived_component_ids = component_db.derived_component_ids(bound_component_id);
    derived_component_ids.push(bound_component_id);
    for derived_component_id in derived_component_ids {
        if let HydratedComponent::Constructor(c) =
            component_db.hydrated_component(derived_component_id, computation_db)
        {
            constructible_db
                .type2constructor_id
                .insert(c.output_type().clone(), derived_component_id);
        }
    }
}
