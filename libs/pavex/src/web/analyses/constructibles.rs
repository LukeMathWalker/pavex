use ahash::{HashMap, HashMapExt, HashSet};
use guppy::graph::PackageGraph;
use miette::{LabeledSpan, NamedSource};
use syn::spanned::Spanned;

use pavex_builder::Lifecycle;

use crate::diagnostic;
use crate::diagnostic::{
    convert_proc_macro_span, convert_rustdoc_span, read_source_file, CompilerDiagnostic,
    LocationExt, SourceSpanExt,
};
use crate::language::{Callable, ResolvedType};
use crate::rustdoc::CrateCollection;
use crate::web::analyses::components::{ComponentDb, ComponentId, HydratedComponent};
use crate::web::analyses::computations::ComputationDb;
use crate::web::analyses::raw_identifiers::RawCallableIdentifiersDb;
use crate::web::analyses::user_components::{UserComponentDb, UserComponentId};

pub(crate) struct ConstructibleDb {
    type2constructor_id: HashMap<ResolvedType, ComponentId>,
}

impl ConstructibleDb {
    pub(crate) fn build(
        component_db: &ComponentDb,
        computation_db: &ComputationDb,
        package_graph: &PackageGraph,
        krate_collection: &CrateCollection,
        user_component_db: &UserComponentDb,
        raw_identifiers_db: &RawCallableIdentifiersDb,
        request_scoped_framework_types: &HashSet<&ResolvedType>,
        diagnostics: &mut Vec<miette::Error>,
    ) -> Self {
        let mut type2constructor_id = HashMap::new();
        for (component_id, component) in component_db.constructors(computation_db) {
            let output = component.output_type();
            type2constructor_id.insert(output.to_owned(), component_id);
        }
        let self_ = Self {
            type2constructor_id,
        };

        for (component_id, _) in component_db.iter() {
            let resolved_component = component_db.hydrated_component(component_id, computation_db);
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

            for (input_index, input) in input_types.into_iter().enumerate() {
                let input = match input.as_ref() {
                    Some(i) => i,
                    None => {
                        continue;
                    }
                };
                if request_scoped_framework_types.contains(input) {
                    continue;
                }
                if self_.get(input).is_some() {
                    continue;
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
        ) -> Option<(String, String, LabeledSpan)> {
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
            .labeled("I do not know how to construct an instance of this input parameter".into());
            let source_path = definition_span.filename.to_str().unwrap();
            Some((source_path.to_string(), source_contents, label))
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
                "I cannot invoke your {component_kind}, `{}`, because it needs an instance \
                of `{unconstructible_type:?}` as input, but I cannot find a constructor for that type.",
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
            .optional_related_error(definition_info.clone().map(
                |(source_path, source_content, label)| {
                    CompilerDiagnostic::builder(
                        NamedSource::new(source_path, source_content),
                        anyhow::anyhow!(""),
                    )
                    .label(label)
                    .build()
                },
            ))
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
