use ahash::HashMap;
use guppy::graph::PackageGraph;
use miette::{miette, NamedSource};
use rustdoc_types::ItemEnum;
use syn::spanned::Spanned;

use crate::diagnostic;
use crate::diagnostic::{
    convert_proc_macro_span, convert_rustdoc_span, AnnotatedSnippet, CompilerDiagnostic,
    LocationExt, SourceSpanExt,
};
use crate::language::{Callable, ResolvedPath};
use crate::rustdoc::CrateCollection;
use crate::web::analyses::raw_identifiers::RawCallableIdentifiersDb;
use crate::web::analyses::resolved_paths::ResolvedPathDb;
use crate::web::analyses::user_components::{UserComponentDb, UserComponentId};
use crate::web::computation::Computation;
use crate::web::interner::Interner;
use crate::web::resolvers::{resolve_callable, CallableResolutionError};

pub(crate) type ComputationId = la_arena::Idx<Computation<'static>>;

pub(crate) struct ComputationDb {
    interner: Interner<Computation<'static>>,
    component_id2callable_id: HashMap<UserComponentId, ComputationId>,
}

impl ComputationDb {
    pub fn build(
        component_db: &UserComponentDb,
        resolved_path_db: &ResolvedPathDb,
        package_graph: &PackageGraph,
        krate_collection: &CrateCollection,
        raw_identifiers_db: &RawCallableIdentifiersDb,
        diagnostics: &mut Vec<miette::Error>,
    ) -> Self {
        let mut self_ = Self {
            interner: Interner::new(),
            component_id2callable_id: Default::default(),
        };
        for (component_id, _) in component_db.iter() {
            let resolved_path = &resolved_path_db[component_id];
            if let Err(e) =
                self_.resolve_callable(krate_collection, resolved_path, Some(component_id))
            {
                Self::capture_diagnostics(
                    e,
                    component_id,
                    component_db,
                    raw_identifiers_db,
                    package_graph,
                    diagnostics,
                );
            }
        }
        self_
    }

    pub(crate) fn resolve_callable(
        &mut self,
        krate_collection: &CrateCollection,
        resolved_path: &ResolvedPath,
        component_id: Option<UserComponentId>,
    ) -> Result<ComputationId, CallableResolutionError> {
        let callable = resolve_callable(krate_collection, resolved_path)?;
        let callable_id = self.interner.get_or_intern(callable.into());
        if let Some(component_id) = component_id {
            self.component_id2callable_id
                .insert(component_id, callable_id);
        }
        Ok(callable_id)
    }

    pub(crate) fn get_or_intern(
        &mut self,
        computation: impl Into<Computation<'static>>,
    ) -> ComputationId {
        self.interner.get_or_intern(computation.into())
    }

    fn capture_diagnostics(
        e: CallableResolutionError,
        component_id: UserComponentId,
        component_db: &UserComponentDb,
        raw_identifiers_db: &RawCallableIdentifiersDb,
        package_graph: &PackageGraph,
        diagnostics: &mut Vec<miette::Error>,
    ) {
        let component = &component_db[component_id];
        let callable_type = component.callable_type();
        let raw_identifier_id = component.raw_callable_identifiers_id();
        let location = raw_identifiers_db.get_location(raw_identifier_id);
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
                    .map(|s| s.labeled(format!("The {callable_type} that we cannot resolve")));
                let diagnostic = CompilerDiagnostic::builder(source, e)
                    .optional_label(label)
                    .help("This is most likely a bug in `pavex` or `rustdoc`.\nPlease file a GitHub issue!".into())
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
                                    ItemEnum::Function(_) => {
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
                                .labeled("I do not know how handle this parameter".into());
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
            CallableResolutionError::UnsupportedCallableKind(_) => {
                let label = diagnostic::get_f_macro_invocation_span(&source, location)
                    .map(|s| s.labeled(format!("It was registered as a {callable_type} here")));
                diagnostics.push(
                    CompilerDiagnostic::builder(source, e)
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
                                    ItemEnum::Function(_) => {
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
                                    .labeled("The output type that I cannot handle".into());
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

impl std::ops::Index<ComputationId> for ComputationDb {
    type Output = Computation<'static>;

    fn index(&self, index: ComputationId) -> &Self::Output {
        &self.interner[index]
    }
}

impl std::ops::Index<UserComponentId> for ComputationDb {
    type Output = Callable;

    fn index(&self, index: UserComponentId) -> &Self::Output {
        match &self[self.component_id2callable_id[&index]] {
            Computation::Callable(c) => c,
            n => {
                dbg!(n);
                unreachable!()
            }
        }
    }
}
