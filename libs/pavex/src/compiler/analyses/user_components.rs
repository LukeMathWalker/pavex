use guppy::graph::PackageGraph;
use miette::{miette, NamedSource};
use syn::spanned::Spanned;

use crate::compiler::analyses::computations::ComputationDb;
use crate::compiler::analyses::raw_user_components::{RawUserComponentDb, RawUserComponentId};
use crate::compiler::analyses::resolved_paths::ResolvedPathDb;
use crate::compiler::analyses::router_validation::validate_router;
use crate::compiler::resolvers::CallableResolutionError;
use crate::diagnostic;
use crate::diagnostic::{
    convert_proc_macro_span, convert_rustdoc_span, AnnotatedSnippet, CompilerDiagnostic,
    LocationExt, SourceSpanExt,
};
use crate::rustdoc::CrateCollection;

pub(crate) struct UserComponentDb {}

impl UserComponentDb {
    /// Process a `Blueprint` and return a `UserComponentDb` that contains all the user components
    /// that have been registered against it.
    pub fn build(
        raw_db: &RawUserComponentDb,
        computation_db: &mut ComputationDb,
        package_graph: &PackageGraph,
        krate_collection: &CrateCollection,
        mut diagnostics: &mut Vec<miette::Error>,
    ) -> Result<Self, ()> {
        /// Exit early if there is at least one error.
        macro_rules! exit_on_errors {
            ($var:ident) => {
                if !$var.is_empty() {
                    return Err(());
                }
            };
        }

        validate_router(&raw_db, &package_graph, &mut diagnostics);
        let resolved_path_db = ResolvedPathDb::build(&raw_db, &package_graph, &mut diagnostics);
        exit_on_errors!(diagnostics);

        Self::resolve_and_intern_paths(
            &resolved_path_db,
            &raw_db,
            computation_db,
            &package_graph,
            &krate_collection,
            &mut diagnostics,
        );
        exit_on_errors!(diagnostics);

        Ok(Self {})
    }

    /// Resolve and intern all the paths in the `ResolvedPathDb`.
    /// Report errors as diagnostics if any of the paths cannot be resolved.
    fn resolve_and_intern_paths(
        resolved_path_db: &ResolvedPathDb,
        raw_db: &RawUserComponentDb,
        computation_db: &mut ComputationDb,
        package_graph: &PackageGraph,
        krate_collection: &CrateCollection,
        diagnostics: &mut Vec<miette::Error>,
    ) {
        for (raw_id, _) in raw_db.iter() {
            let resolved_path = &resolved_path_db[raw_id];
            if let Err(e) =
                computation_db.resolve_and_intern(krate_collection, resolved_path, Some(raw_id))
            {
                Self::cannot_resolve_path(e, raw_id, raw_db, package_graph, diagnostics);
            }
        }
    }

    fn cannot_resolve_path(
        e: CallableResolutionError,
        component_id: RawUserComponentId,
        component_db: &RawUserComponentDb,
        package_graph: &PackageGraph,
        diagnostics: &mut Vec<miette::Error>,
    ) {
        let location = component_db.get_location(component_id);
        let component = &component_db[component_id];
        let callable_type = component.callable_type();
        let source = match location.source_file(package_graph) {
            Ok(source) => source,
            Err(e) => {
                diagnostics.push(e.into());
                return;
            }
        };
        match e {
            CallableResolutionError::UnknownCallable(_) => {
                let label = diagnostic::get_f_macro_invocation_span(&source, location)
                    .map(|s| s.labeled(format!("The {callable_type} that we can't resolve")));
                let diagnostic = CompilerDiagnostic::builder(source, e)
                    .optional_label(label)
                    .help("Check that the path is spelled correctly and that the function (or method) is marked as `pub`.".into())
                    .build();
                diagnostics.push(diagnostic.into());
            }
            CallableResolutionError::ParameterResolutionError(ref inner_error) => {
                let definition_snippet = {
                    if let Some(definition_span) = &inner_error.callable_item.span {
                        match diagnostic::read_source_file(
                            &definition_span.filename,
                            &package_graph.workspace(),
                        ) {
                            Ok(source_contents) => {
                                let span = convert_rustdoc_span(
                                    &source_contents,
                                    definition_span.to_owned(),
                                );
                                let span_contents =
                                    &source_contents[span.offset()..(span.offset() + span.len())];
                                let input = match &inner_error.callable_item.inner {
                                    rustdoc_types::ItemEnum::Function(_) => {
                                        if let Ok(item) = syn::parse_str::<syn::ItemFn>(span_contents) {
                                            let mut inputs = item.sig.inputs.iter();
                                            inputs.nth(inner_error.parameter_index).cloned()
                                        } else if let Ok(item) =
                                            syn::parse_str::<syn::ImplItemMethod>(span_contents)
                                        {
                                            let mut inputs = item.sig.inputs.iter();
                                            inputs.nth(inner_error.parameter_index).cloned()
                                        } else {
                                            panic!(
                                                "Could not parse as a function or method:\n{span_contents}"
                                            )
                                        }
                                    }
                                    _ => unreachable!(),
                                }
                                    .unwrap();
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
                                .labeled("I don't know how handle this parameter".into());
                                let source_path = definition_span.filename.to_str().unwrap();
                                Some(AnnotatedSnippet::new(
                                    NamedSource::new(source_path, source_contents),
                                    label,
                                ))
                            }
                            Err(e) => {
                                tracing::warn!("Could not read source file: {:?}", e);
                                None
                            }
                        }
                    } else {
                        None
                    }
                };
                let label = diagnostic::get_f_macro_invocation_span(&source, location)
                    .map(|s| s.labeled(format!("The {callable_type} was registered here")));
                let diagnostic = CompilerDiagnostic::builder(source, e.clone())
                    .optional_label(label)
                    .optional_additional_annotated_snippet(definition_snippet)
                    .build();
                diagnostics.push(diagnostic.into());
            }
            CallableResolutionError::UnsupportedCallableKind(ref inner_error) => {
                let label = diagnostic::get_f_macro_invocation_span(&source, location)
                    .map(|s| s.labeled(format!("It was registered as a {callable_type} here")));
                let message = format!("I can work with functions and static methods, but `{}` is neither.\nIt is {} and I don't know how to use it as a {}.", inner_error.import_path, inner_error.item_kind, callable_type);
                let error = anyhow::anyhow!(e).context(message);
                diagnostics.push(
                    CompilerDiagnostic::builder(source, error)
                        .optional_label(label)
                        .build()
                        .into(),
                );
            }
            CallableResolutionError::OutputTypeResolutionError(ref inner_error) => {
                let annotated_snippet = {
                    if let Some(definition_span) = &inner_error.callable_item.span {
                        match diagnostic::read_source_file(
                            &definition_span.filename,
                            &package_graph.workspace(),
                        ) {
                            Ok(source_contents) => {
                                let span = convert_rustdoc_span(
                                    &source_contents,
                                    definition_span.to_owned(),
                                );
                                let span_contents = source_contents
                                    [span.offset()..(span.offset() + span.len())]
                                    .to_string();
                                let output = match &inner_error.callable_item.inner {
                                    rustdoc_types::ItemEnum::Function(_) => {
                                        if let Ok(item) =
                                            syn::parse_str::<syn::ItemFn>(&span_contents)
                                        {
                                            item.sig.output
                                        } else if let Ok(item) =
                                            syn::parse_str::<syn::ImplItemMethod>(&span_contents)
                                        {
                                            item.sig.output
                                        } else {
                                            panic!(
                                                "Could not parse as a function or method:\n{span_contents}"
                                            )
                                        }
                                    }
                                    _ => unreachable!(),
                                };
                                match output {
                                    syn::ReturnType::Default => None,
                                    syn::ReturnType::Type(_, type_) => Some(type_.span()),
                                }
                                .map(|s| {
                                    let s = convert_proc_macro_span(&span_contents, s);
                                    let label = miette::SourceSpan::new(
                                        // We must shift the offset forward because it's the
                                        // offset from the beginning of the file slice that
                                        // we deserialized, instead of the entire file
                                        (s.offset() + span.offset()).into(),
                                        s.len().into(),
                                    )
                                    .labeled("The output type that I can't handle".into());
                                    AnnotatedSnippet::new(
                                        NamedSource::new(
                                            definition_span.filename.to_str().unwrap(),
                                            source_contents,
                                        ),
                                        label,
                                    )
                                })
                            }
                            Err(e) => {
                                tracing::warn!("Could not read source file: {:?}", e);
                                None
                            }
                        }
                    } else {
                        None
                    }
                };

                let label = diagnostic::get_f_macro_invocation_span(&source, location)
                    .map(|s| s.labeled(format!("The {callable_type} was registered here")));
                diagnostics.push(
                    CompilerDiagnostic::builder(source, e.clone())
                        .optional_label(label)
                        .optional_additional_annotated_snippet(annotated_snippet)
                        .build()
                        .into(),
                )
            }
            CallableResolutionError::CannotGetCrateData(_) => {
                diagnostics.push(miette!(e));
            }
        }
    }
}
