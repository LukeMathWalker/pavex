use ahash::HashMap;
use guppy::graph::PackageGraph;
use indexmap::IndexSet;
use miette::{miette, NamedSource};
use std::collections::BTreeMap;
use syn::spanned::Spanned;

use pavex_bp_schema::{
    Blueprint, CloningStrategy, Lifecycle, Lint, LintSetting, Location, RawCallableIdentifiers,
};

use crate::compiler::analyses::computations::ComputationDb;
use crate::compiler::analyses::user_components::raw_db::RawUserComponentDb;
use crate::compiler::analyses::user_components::resolved_paths::ResolvedPathDb;
use crate::compiler::analyses::user_components::router::Router;
use crate::compiler::analyses::user_components::{ScopeGraph, UserComponent, UserComponentId};
use crate::compiler::interner::Interner;
use crate::compiler::resolvers::CallableResolutionError;
use crate::diagnostic::{
    convert_proc_macro_span, convert_rustdoc_span, AnnotatedSnippet, CompilerDiagnostic,
    OptionalSourceSpanExt, SourceSpanExt,
};
use crate::rustdoc::CrateCollection;
use crate::{diagnostic, try_source};

/// A database that contains all the user components that have been registered against the
/// `Blueprint` for the application.
///
/// For each component, we keep track of:
/// - the source code location where it was registered (for error reporting purposes);
/// - the lifecycle of the component;
/// - the scope that the component belongs to.
///
/// Some basic validation has been carried out:
/// - the callable associated to each component has been resolved and added to the
///   provided [`ComputationDb`].
/// - there are no conflicting routes.
#[derive(Debug)]
pub struct UserComponentDb {
    component_interner: Interner<UserComponent>,
    identifiers_interner: Interner<RawCallableIdentifiers>,
    /// Associate each user-registered component with the location it was
    /// registered at against the `Blueprint` in the user's source code.
    ///
    /// Invariants: there is an entry for every single user component.
    id2locations: HashMap<UserComponentId, Location>,
    /// Associate each user-registered component with its lifecycle.
    ///
    /// Invariants: there is an entry for every single user component.
    id2lifecycle: HashMap<UserComponentId, Lifecycle>,
    /// Associate each user-registered component with its lint overrides, if any.
    /// If there is no entry for a component, there are no overrides.
    id2lints: HashMap<UserComponentId, BTreeMap<Lint, LintSetting>>,
    /// For each constructor component, determine if it can be cloned or not.
    ///
    /// Invariants: there is an entry for every constructor.
    constructor_id2cloning_strategy: HashMap<UserComponentId, CloningStrategy>,
    /// Associate each request handler with the ordered list of middlewares that wrap around it.
    ///
    /// Invariants: there is an entry for every single request handler.
    handler_id2middleware_ids: HashMap<UserComponentId, Vec<UserComponentId>>,
    /// Associate each request handler with the ordered list of error observers
    /// that must be invoked when an error occurs while handling a request.
    ///
    /// Invariants: there is an entry for every single request handler.
    handler_id2error_observer_ids: HashMap<UserComponentId, Vec<UserComponentId>>,
    scope_graph: ScopeGraph,
}

impl UserComponentDb {
    /// Process a `Blueprint` and return a `UserComponentDb` that contains all the user components
    /// that have been registered against it.
    ///
    /// The callable associated to each component will be resolved and added to the
    /// provided [`ComputationDb`].
    #[tracing::instrument(name = "Build user component database", skip_all)]
    pub(crate) fn build(
        bp: &Blueprint,
        computation_db: &mut ComputationDb,
        package_graph: &PackageGraph,
        krate_collection: &CrateCollection,
        diagnostics: &mut Vec<miette::Error>,
    ) -> Result<(Router, Self), ()> {
        /// Exit early if there is at least one error.
        macro_rules! exit_on_errors {
            ($var:ident) => {
                if !$var.is_empty() {
                    return Err(());
                }
            };
        }

        let (raw_db, scope_graph) = RawUserComponentDb::build(bp, package_graph, diagnostics);
        let resolved_path_db = ResolvedPathDb::build(&raw_db, package_graph, diagnostics);
        let router = Router::new(&raw_db, &scope_graph, package_graph, diagnostics)?;
        exit_on_errors!(diagnostics);

        precompute_crate_docs(krate_collection, &resolved_path_db, diagnostics);
        exit_on_errors!(diagnostics);

        Self::resolve_and_intern_paths(
            &resolved_path_db,
            &raw_db,
            computation_db,
            package_graph,
            krate_collection,
            diagnostics,
        );
        exit_on_errors!(diagnostics);

        let RawUserComponentDb {
            component_interner,
            id2locations,
            id2lints,
            constructor_id2cloning_strategy,
            id2lifecycle,
            identifiers_interner,
            handler_id2middleware_ids,
            handler_id2error_observer_ids,
            fallback_id2path_prefix: _,
        } = raw_db;

        Ok((
            router,
            Self {
                component_interner,
                identifiers_interner,
                id2locations,
                constructor_id2cloning_strategy,
                id2lifecycle,
                handler_id2middleware_ids,
                handler_id2error_observer_ids,
                scope_graph,
                id2lints,
            },
        ))
    }

    /// Iterate over all the user components in the database, returning their id and the associated
    /// `UserComponent`.
    pub fn iter(
        &self,
    ) -> impl Iterator<Item = (UserComponentId, &UserComponent)> + ExactSizeIterator + DoubleEndedIterator
    {
        self.component_interner.iter()
    }

    /// Iterate over all the constructor components in the database, returning their id and the
    /// associated `UserComponent`.
    pub fn constructors(
        &self,
    ) -> impl Iterator<Item = (UserComponentId, &UserComponent)> + DoubleEndedIterator {
        self.component_interner
            .iter()
            .filter(|(_, c)| matches!(c, UserComponent::Constructor { .. }))
    }

    /// Iterate over all the request handler components in the database, returning their id and the
    /// associated `UserComponent`.
    ///
    /// It returns both routes (i.e. handlers that are registered against a specific path and method
    /// guard) and fallback handlers.
    pub fn request_handlers(
        &self,
    ) -> impl Iterator<Item = (UserComponentId, &UserComponent)> + DoubleEndedIterator {
        self.component_interner.iter().filter(|(_, c)| {
            matches!(
                c,
                UserComponent::RequestHandler { .. } | UserComponent::Fallback { .. }
            )
        })
    }

    /// Iterate over all the wrapping middleware components in the database, returning their id and the
    /// associated `UserComponent`.
    pub fn wrapping_middlewares(
        &self,
    ) -> impl Iterator<Item = (UserComponentId, &UserComponent)> + DoubleEndedIterator {
        self.component_interner
            .iter()
            .filter(|(_, c)| matches!(c, UserComponent::WrappingMiddleware { .. }))
    }

    /// Iterate over all the error observer components in the database, returning their id and the
    /// associated `UserComponent`.
    pub fn error_observers(
        &self,
    ) -> impl Iterator<Item = (UserComponentId, &UserComponent)> + DoubleEndedIterator {
        self.component_interner
            .iter()
            .filter(|(_, c)| matches!(c, UserComponent::ErrorObserver { .. }))
    }

    /// Return the lifecycle of the component with the given id.
    pub fn get_lifecycle(&self, id: UserComponentId) -> Lifecycle {
        self.id2lifecycle[&id]
    }

    /// Return the location where the component with the given id was registered against the
    /// application blueprint.
    pub fn get_location(&self, id: UserComponentId) -> &Location {
        &self.id2locations[&id]
    }

    /// Return the cloning strategy of the component with the given id.
    /// This is going to be `Some(..)` for constructor components, and `None` for all other components.
    pub fn get_cloning_strategy(&self, id: UserComponentId) -> Option<&CloningStrategy> {
        self.constructor_id2cloning_strategy.get(&id)
    }

    /// Return the scope tree that was built from the application blueprint.
    pub fn scope_graph(&self) -> &ScopeGraph {
        &self.scope_graph
    }

    /// Return the raw callable identifiers associated to the user component with the given id.
    ///
    /// This can be used to recover the original import path passed by the user when registering
    /// this component, primarily for error reporting purposes.
    pub fn get_raw_callable_identifiers(&self, id: UserComponentId) -> &RawCallableIdentifiers {
        let raw_id = self.component_interner[id].raw_callable_identifiers_id();
        &self.identifiers_interner[raw_id]
    }

    /// Return the ids of the middlewares that wrap around the request handler with the given id.
    ///
    /// It panics if the component with the given id is not a request handler.
    pub fn get_middleware_ids(&self, id: UserComponentId) -> &[UserComponentId] {
        &self.handler_id2middleware_ids[&id]
    }

    /// Return the lint overrides for this component, if any.
    pub fn get_lints(&self, id: UserComponentId) -> Option<&BTreeMap<Lint, LintSetting>> {
        self.id2lints.get(&id)
    }

    /// Return the ids of the error observers that must be invoked when something goes wrong
    /// in the request processing pipeline for this handler.
    ///
    /// It panics if the component with the given id is not a request handler.
    pub fn get_error_observer_ids(&self, id: UserComponentId) -> &[UserComponentId] {
        &self.handler_id2error_observer_ids[&id]
    }
}

/// We try to batch together the computation of the JSON documentation for all the crates that,
/// based on the information we have so far, will be needed to generate the application code.
///
/// This is not strictly necessary, but it can turn out to be a significant performance improvement
/// for projects that pull in a lot of dependencies in the signature of their components.
fn precompute_crate_docs(
    krate_collection: &CrateCollection,
    resolved_path_db: &ResolvedPathDb,
    diagnostics: &mut Vec<miette::Error>,
) {
    let mut package_ids = IndexSet::new();
    for (_, path) in resolved_path_db.iter() {
        path.collect_package_ids(&mut package_ids);
    }
    if let Err(e) = krate_collection.bootstrap_collection(package_ids.into_iter().cloned()) {
        let e = anyhow::anyhow!(e).context(
            "I failed to compute the JSON documentation for one or more crates in the workspace.",
        );
        // TODO: This throws away the error history, it sucks.
        diagnostics.push(miette!(e));
    }
}

impl UserComponentDb {
    /// Resolve and intern all the paths in the `ResolvedPathDb`.
    /// Report errors as diagnostics if any of the paths cannot be resolved.
    #[tracing::instrument(name = "Resolve and intern paths", skip_all, level = "trace")]
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
        component_id: UserComponentId,
        component_db: &RawUserComponentDb,
        package_graph: &PackageGraph,
        diagnostics: &mut Vec<miette::Error>,
    ) {
        let location = component_db.get_location(component_id);
        let component = &component_db[component_id];
        let callable_type = component.callable_type();
        let source = try_source!(location, package_graph, diagnostics);
        match e {
            CallableResolutionError::UnknownCallable(_) => {
                let label = source
                    .as_ref()
                    .map(|source| {
                        diagnostic::get_f_macro_invocation_span(&source, location)
                            .labeled(format!("The {callable_type} that we can't resolve"))
                    })
                    .flatten();
                let diagnostic = CompilerDiagnostic::builder(e).optional_source(source)
                    .optional_label(label)
                    .help("Check that the path is spelled correctly and that the function (or method) is public.".into())
                    .build();
                diagnostics.push(diagnostic.into());
            }
            CallableResolutionError::InputParameterResolutionError(ref inner_error) => {
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
                                            syn::parse_str::<syn::ImplItemFn>(span_contents)
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
                let label = source
                    .as_ref()
                    .map(|source| {
                        diagnostic::get_f_macro_invocation_span(&source, location)
                            .labeled(format!("The {callable_type} was registered here"))
                    })
                    .flatten();
                let diagnostic = CompilerDiagnostic::builder(e.clone())
                    .optional_source(source)
                    .optional_label(label)
                    .optional_additional_annotated_snippet(definition_snippet)
                    .build();
                diagnostics.push(diagnostic.into());
            }
            CallableResolutionError::UnsupportedCallableKind(ref inner_error) => {
                let label = source
                    .as_ref()
                    .map(|source| {
                        diagnostic::get_f_macro_invocation_span(&source, location)
                            .labeled(format!("It was registered as a {callable_type} here"))
                    })
                    .flatten();
                let message = format!("I can work with functions and methods, but `{}` is neither.\nIt is {} and I don't know how to use it as a {}.", inner_error.import_path, inner_error.item_kind, callable_type);
                let error = anyhow::anyhow!(e).context(message);
                diagnostics.push(
                    CompilerDiagnostic::builder(error)
                        .optional_source(source)
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
                                            syn::parse_str::<syn::ImplItemFn>(&span_contents)
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

                let label = source
                    .as_ref()
                    .map(|source| {
                        diagnostic::get_f_macro_invocation_span(&source, location)
                            .labeled(format!("The {callable_type} was registered here"))
                    })
                    .flatten();
                diagnostics.push(
                    CompilerDiagnostic::builder(e.clone())
                        .optional_source(source)
                        .optional_label(label)
                        .optional_additional_annotated_snippet(annotated_snippet)
                        .build()
                        .into(),
                )
            }
            CallableResolutionError::CannotGetCrateData(_) => {
                diagnostics.push(miette!(e));
            }
            CallableResolutionError::GenericParameterResolutionError(_) => {
                let label = source
                    .as_ref()
                    .map(|source| {
                        diagnostic::get_f_macro_invocation_span(&source, location)
                            .labeled(format!("The {callable_type} was registered here"))
                    })
                    .flatten();
                let diagnostic = CompilerDiagnostic::builder(e)
                    .optional_source(source)
                    .optional_label(label)
                    .build();
                diagnostics.push(diagnostic.into());
            }
        }
    }
}

impl std::ops::Index<UserComponentId> for UserComponentDb {
    type Output = UserComponent;

    fn index(&self, index: UserComponentId) -> &Self::Output {
        &self.component_interner[index]
    }
}

impl std::ops::Index<&UserComponent> for UserComponentDb {
    type Output = UserComponentId;

    fn index(&self, index: &UserComponent) -> &Self::Output {
        &self.component_interner[index]
    }
}
