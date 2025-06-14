use pavex_bp_schema::{
    Blueprint, Callable, CloningStrategy, Component, ConfigType, Constructor, CreatedAt, Domain,
    ErrorHandler, ErrorObserver, Fallback, Import, Lifecycle, Location, NestedBlueprint,
    PathPrefix, PostProcessingMiddleware, PreProcessingMiddleware, PrebuiltType, RawIdentifiers,
    Route, RoutesImport, WrappingMiddleware,
};

use super::auxiliary::AuxiliaryData;
use super::imports::{ImportKind, UnresolvedImport};
use super::{ErrorHandlerTarget, UserComponentId, UserComponentSource};
use crate::compiler::analyses::domain::DomainGuard;
use crate::compiler::analyses::user_components::router_key::RouterKey;
use crate::compiler::analyses::user_components::scope_graph::ScopeGraphBuilder;
use crate::compiler::analyses::user_components::{ScopeGraph, ScopeId, UserComponent};
use crate::compiler::app::PAVEX_VERSION;
use crate::compiler::component::DefaultStrategy;
use crate::rustdoc::AnnotationCoordinates;

/// A unique identifier for a `RawCallableIdentifiers`.
pub type RawIdentifierId = la_arena::Idx<RawIdentifiers>;

/// Process a [`Blueprint`], populating [`AuxiliaryData`] with all its registered components.
///
/// It returns a [`ScopeGraph`] too.
pub(super) fn process_blueprint(
    bp: &Blueprint,
    aux: &mut AuxiliaryData,
    diagnostics: &crate::diagnostic::DiagnosticSink,
) -> ScopeGraphBuilder {
    let mut scope_graph_builder = ScopeGraph::builder(bp.creation_location.clone());
    let root_scope_id = scope_graph_builder.root_scope_id();
    // The middleware chain that will wrap around all the request handlers in the current scope.
    // We discover and add more middlewares to this chain as we process the root blueprint and
    // its nested blueprints.
    // By default, the middleware chain is empty.
    let mut current_middleware_chain = Vec::new();
    // The error observers that will be invoked if there is an error while handling a request
    // in the current scope.
    // We discover and add more error observers to this chain as we process the root blueprint
    // and its nested blueprints.
    // By default, the error observer chain is empty.
    let mut current_observer_chain = Vec::new();

    let mut processing_queue = Vec::new();

    _process_blueprint(
        bp,
        aux,
        root_scope_id,
        None,
        None,
        &mut scope_graph_builder,
        &mut current_middleware_chain,
        &mut current_observer_chain,
        true,
        &mut processing_queue,
        diagnostics,
    );

    while let Some(item) = processing_queue.pop() {
        let QueueItem {
            parent_scope_id,
            nested_bp,
            parent_path_prefix,
            parent_domain_guard,
            mut current_middleware_chain,
            mut current_observer_chain,
        } = item;
        let nested_scope_id =
            scope_graph_builder.add_scope(parent_scope_id, Some(nested_bp.nested_at.clone()));
        let Ok((current_prefix, current_domain)) =
            process_nesting_constraints(aux, nested_bp, diagnostics)
        else {
            continue;
        };

        let path_prefix = match parent_path_prefix {
            Some(prefix) => Some(format!(
                "{}{}",
                prefix,
                current_prefix.as_deref().unwrap_or("")
            )),
            None => current_prefix.clone(),
        };
        let domain_guard = match current_domain {
            Some(domain) => Some(domain),
            None => parent_domain_guard,
        };

        _process_blueprint(
            &nested_bp.blueprint,
            aux,
            nested_scope_id,
            domain_guard,
            path_prefix.as_deref(),
            &mut scope_graph_builder,
            &mut current_middleware_chain,
            &mut current_observer_chain,
            false,
            &mut processing_queue,
            diagnostics,
        );
    }

    #[cfg(debug_assertions)]
    aux.check_invariants();

    scope_graph_builder
}

/// Used in [`process_blueprint`] to keep track of the nested blueprints that we still
/// need to process.
struct QueueItem<'a> {
    parent_scope_id: ScopeId,
    parent_path_prefix: Option<String>,
    parent_domain_guard: Option<DomainGuard>,
    nested_bp: &'a NestedBlueprint,
    current_middleware_chain: Vec<UserComponentId>,
    current_observer_chain: Vec<UserComponentId>,
}

/// Register with [`AuxiliaryData`] all the user components that have been
/// registered against the provided `Blueprint`.
/// All components are associated with or nested under the provided `current_scope_id`.
///
/// If `path_prefix` is `Some`, then it is prepended to the path of each route
/// in `Blueprint`.
fn _process_blueprint<'a>(
    bp: &'a Blueprint,
    aux: &mut AuxiliaryData,
    current_scope_id: ScopeId,
    domain_guard: Option<DomainGuard>,
    path_prefix: Option<&str>,
    scope_graph_builder: &mut ScopeGraphBuilder,
    current_middleware_chain: &mut Vec<UserComponentId>,
    current_observer_chain: &mut Vec<UserComponentId>,
    is_root: bool,
    bp_queue: &mut Vec<QueueItem<'a>>,
    diagnostics: &crate::diagnostic::DiagnosticSink,
) {
    let mut fallback: Option<&Fallback> = None;
    for component in &bp.components {
        match component {
            Component::Constructor(c) => {
                process_constructor(aux, c, current_scope_id);
            }
            Component::WrappingMiddleware(w) => {
                process_middleware(
                    aux,
                    w,
                    current_scope_id,
                    current_middleware_chain,
                    scope_graph_builder,
                );
            }
            Component::PreProcessingMiddleware(p) => {
                process_pre_processing_middleware(
                    aux,
                    p,
                    current_scope_id,
                    current_middleware_chain,
                    scope_graph_builder,
                );
            }
            Component::PostProcessingMiddleware(p) => {
                process_post_processing_middleware(
                    aux,
                    p,
                    current_scope_id,
                    current_middleware_chain,
                    scope_graph_builder,
                );
            }
            Component::Route(r) => process_route(
                aux,
                r,
                current_middleware_chain,
                current_observer_chain,
                current_scope_id,
                domain_guard.clone(),
                path_prefix,
                scope_graph_builder,
                diagnostics,
            ),
            Component::FallbackRequestHandler(f) => {
                fallback = Some(f);
            }
            Component::NestedBlueprint(b) => {
                bp_queue.push(QueueItem {
                    parent_scope_id: current_scope_id,
                    nested_bp: b,
                    parent_path_prefix: path_prefix.map(|s| s.to_owned()),
                    parent_domain_guard: domain_guard.clone(),
                    current_middleware_chain: current_middleware_chain.clone(),
                    current_observer_chain: current_observer_chain.clone(),
                });
            }
            Component::ErrorObserver(eo) => {
                process_error_observer(aux, eo, current_scope_id, current_observer_chain);
            }
            Component::ErrorHandler(eh) => {
                process_error_handler(aux, eh, current_scope_id);
            }
            Component::PrebuiltType(si) => {
                process_prebuilt_type(aux, si, current_scope_id);
            }
            Component::ConfigType(t) => {
                process_config_type(aux, t, current_scope_id);
            }
            Component::RoutesImport(RoutesImport {
                sources,
                created_at,
                registered_at,
            })
            | Component::Import(Import {
                sources,
                created_at,
                registered_at,
            }) => {
                let kind = if matches!(component, Component::Import(_)) {
                    ImportKind::OrderIndependentComponents
                } else {
                    ImportKind::Routes {
                        path_prefix: path_prefix.map(ToOwned::to_owned),
                        domain_guard: domain_guard.clone(),
                        observer_chain: current_observer_chain.clone(),
                        middleware_chain: current_middleware_chain.clone(),
                    }
                };
                aux.imports.push(UnresolvedImport {
                    scope_id: current_scope_id,
                    sources: sources.to_owned(),
                    created_at: created_at.to_owned(),
                    registered_at: registered_at.to_owned(),
                    kind,
                });
            }
        }
    }

    let fallback = fallback.cloned().or_else(|| {
        // We need to have a top-level fallback handler.
        // If the user hasn't registered one against the top-level blueprint,
        // we use the framework's default one.
        is_root.then(|| Fallback {
            coordinates: pavex_reflection::AnnotationCoordinates {
                id: "DEFAULT_FALLBACK".into(),
                created_at: CreatedAt {
                    package_name: "pavex".to_owned(),
                    package_version: PAVEX_VERSION.to_owned(),
                    module_path: "pavex::router".to_owned(),
                },
                macro_name: "fallback".into(),
            },
            // TODO: We should have a better location for framework-provided
            //   components.
            registered_at: bp.creation_location.clone(),
            error_handler: None,
        })
    });
    if let Some(fallback) = fallback {
        process_fallback(
            aux,
            &fallback,
            path_prefix,
            domain_guard,
            current_middleware_chain,
            current_observer_chain,
            current_scope_id,
            scope_graph_builder,
        );
    }
}

/// Process a route that has been
/// registered against the provided `Blueprint`, including its error handler
/// (if present).
fn process_route(
    aux: &mut AuxiliaryData,
    route: &Route,
    current_middleware_chain: &[UserComponentId],
    current_observer_chain: &[UserComponentId],
    current_scope_id: ScopeId,
    domain_guard: Option<DomainGuard>,
    path_prefix: Option<&str>,
    scope_graph_builder: &mut ScopeGraphBuilder,
    _diagnostics: &crate::diagnostic::DiagnosticSink,
) {
    const ROUTE_LIFECYCLE: Lifecycle = Lifecycle::RequestScoped;

    let annotation_coordinates = AnnotationCoordinates {
        id: route.coordinates.id.clone(),
        created_at: route.coordinates.created_at.clone(),
    };
    let coordinates_id = aux
        .annotation_coordinates_interner
        .get_or_intern(annotation_coordinates);
    let route_scope_id = scope_graph_builder.add_scope(current_scope_id, None);

    // For routes registered directly via bp.route(), we create a partial `RouterKey`.
    let router_key = RouterKey {
        path: path_prefix.unwrap_or_default().to_owned(), // Will be finalized later, during annotation resolution
        method_guard: pavex_bp_schema::MethodGuard::Any, // Will be filled in during annotation resolution
        domain_guard,
    };
    let component = UserComponent::RequestHandler {
        router_key,
        source: UserComponentSource::AnnotationCoordinates(coordinates_id),
    };
    let request_handler_id = aux.intern_component(
        component,
        route_scope_id,
        ROUTE_LIFECYCLE,
        route.registered_at.clone().into(),
    );

    aux.handler_id2middleware_ids
        .insert(request_handler_id, current_middleware_chain.to_owned());
    aux.handler_id2error_observer_ids
        .insert(request_handler_id, current_observer_chain.to_owned());

    // Path validation will happen during annotation resolution

    process_component_specific_error_handler(
        aux,
        &route.error_handler,
        ROUTE_LIFECYCLE,
        current_scope_id,
        request_handler_id,
    );
}

/// Process the fallback that has been
/// registered against the provided `Blueprint`, including its error handler
/// (if present).
fn process_fallback(
    aux: &mut AuxiliaryData,
    fallback: &Fallback,
    path_prefix: Option<&str>,
    domain_guard: Option<DomainGuard>,
    current_middleware_chain: &[UserComponentId],
    current_observer_chain: &[UserComponentId],
    current_scope_id: ScopeId,
    scope_graph_builder: &mut ScopeGraphBuilder,
) {
    const ROUTE_LIFECYCLE: Lifecycle = Lifecycle::RequestScoped;

    let annotation_coordinates = AnnotationCoordinates {
        id: fallback.coordinates.id.clone(),
        created_at: fallback.coordinates.created_at.clone(),
    };
    let coordinates_id = aux
        .annotation_coordinates_interner
        .get_or_intern(annotation_coordinates);
    let route_scope_id = scope_graph_builder.add_scope(current_scope_id, None);
    let component = UserComponent::Fallback {
        source: coordinates_id,
    };
    let fallback_id = aux.intern_component(
        component,
        route_scope_id,
        ROUTE_LIFECYCLE,
        fallback.registered_at.clone().into(),
    );

    aux.handler_id2middleware_ids
        .insert(fallback_id, current_middleware_chain.to_owned());
    aux.handler_id2error_observer_ids
        .insert(fallback_id, current_observer_chain.to_owned());
    aux.fallback_id2path_prefix
        .insert(fallback_id, path_prefix.map(|s| s.to_owned()));
    aux.fallback_id2domain_guard
        .insert(fallback_id, domain_guard);

    process_component_specific_error_handler(
        aux,
        &fallback.error_handler,
        ROUTE_LIFECYCLE,
        current_scope_id,
        fallback_id,
    );
}

/// Process a middleware that has been
/// registered against the provided `Blueprint`, including its error handler
/// (if present).
fn process_middleware(
    aux: &mut AuxiliaryData,
    middleware: &WrappingMiddleware,
    current_scope_id: ScopeId,
    current_middleware_chain: &mut Vec<UserComponentId>,
    scope_graph_builder: &mut ScopeGraphBuilder,
) {
    const MIDDLEWARE_LIFECYCLE: Lifecycle = Lifecycle::RequestScoped;

    let middleware_scope_id = scope_graph_builder.add_scope(current_scope_id, None);
    let annotation_coordinates = AnnotationCoordinates {
        id: middleware.coordinates.id.clone(),
        created_at: middleware.coordinates.created_at.clone(),
    };
    let coordinates_id = aux
        .annotation_coordinates_interner
        .get_or_intern(annotation_coordinates);
    let component = UserComponent::WrappingMiddleware {
        source: coordinates_id,
    };
    let component_id = aux.intern_component(
        component,
        middleware_scope_id,
        MIDDLEWARE_LIFECYCLE,
        middleware.registered_at.clone().into(),
    );
    current_middleware_chain.push(component_id);

    process_component_specific_error_handler(
        aux,
        &middleware.error_handler,
        MIDDLEWARE_LIFECYCLE,
        current_scope_id,
        component_id,
    );
}

/// Process a pre-processing middleware that has been
/// registered against the provided `Blueprint`, including its error handler
/// (if present).
fn process_pre_processing_middleware(
    aux: &mut AuxiliaryData,
    middleware: &PreProcessingMiddleware,
    current_scope_id: ScopeId,
    current_middleware_chain: &mut Vec<UserComponentId>,
    scope_graph_builder: &mut ScopeGraphBuilder,
) {
    const MIDDLEWARE_LIFECYCLE: Lifecycle = Lifecycle::RequestScoped;

    let middleware_scope_id = scope_graph_builder.add_scope(current_scope_id, None);
    let annotation_coordinates = AnnotationCoordinates {
        id: middleware.coordinates.id.clone(),
        created_at: middleware.coordinates.created_at.clone(),
    };
    let coordinates_id = aux
        .annotation_coordinates_interner
        .get_or_intern(annotation_coordinates);
    let component = UserComponent::PreProcessingMiddleware {
        source: coordinates_id,
    };
    let component_id = aux.intern_component(
        component,
        middleware_scope_id,
        MIDDLEWARE_LIFECYCLE,
        middleware.registered_at.clone().into(),
    );
    current_middleware_chain.push(component_id);

    process_component_specific_error_handler(
        aux,
        &middleware.error_handler,
        MIDDLEWARE_LIFECYCLE,
        current_scope_id,
        component_id,
    );
}

/// Process a post-processing middleware that has been
/// registered against the provided `Blueprint`, including its error handler
/// (if present).
fn process_post_processing_middleware(
    aux: &mut AuxiliaryData,
    middleware: &PostProcessingMiddleware,
    current_scope_id: ScopeId,
    current_middleware_chain: &mut Vec<UserComponentId>,
    scope_graph_builder: &mut ScopeGraphBuilder,
) {
    const MIDDLEWARE_LIFECYCLE: Lifecycle = Lifecycle::RequestScoped;

    let middleware_scope_id = scope_graph_builder.add_scope(current_scope_id, None);
    let annotation_coordinates = AnnotationCoordinates {
        id: middleware.coordinates.id.clone(),
        created_at: middleware.coordinates.created_at.clone(),
    };
    let coordinates_id = aux
        .annotation_coordinates_interner
        .get_or_intern(annotation_coordinates);
    let component = UserComponent::PostProcessingMiddleware {
        source: coordinates_id,
    };
    let component_id = aux.intern_component(
        component,
        middleware_scope_id,
        MIDDLEWARE_LIFECYCLE,
        middleware.registered_at.clone().into(),
    );
    current_middleware_chain.push(component_id);

    process_component_specific_error_handler(
        aux,
        &middleware.error_handler,
        MIDDLEWARE_LIFECYCLE,
        current_scope_id,
        component_id,
    );
}

/// Process a constructor that has been
/// registered against the provided `Blueprint`, including its error handler
/// (if present).
/// It is associated with or nested under the provided `current_scope_id`.
fn process_constructor(
    aux: &mut AuxiliaryData,
    constructor: &Constructor,
    current_scope_id: ScopeId,
) {
    let identifiers_id = aux
        .identifiers_interner
        .get_or_intern(constructor.constructor.callable.clone());
    let component = UserComponent::Constructor {
        source: identifiers_id.into(),
    };
    let lifecycle = constructor.lifecycle;
    let constructor_id = aux.intern_component(
        component,
        current_scope_id,
        lifecycle,
        constructor.constructor.registered_at.clone().into(),
    );
    aux.id2cloning_strategy.insert(
        constructor_id,
        constructor
            .cloning_strategy
            .unwrap_or(CloningStrategy::NeverClone),
    );
    if !constructor.lints.is_empty() {
        aux.id2lints
            .insert(constructor_id, constructor.lints.clone());
    }

    process_legacy_component_specific_error_handler(
        aux,
        &constructor.error_handler,
        lifecycle,
        current_scope_id,
        constructor_id,
    );
}

/// Process an error observer that has been
/// registered against the provided `Blueprint`.
/// It is associated with or nested under the provided `current_scope_id`.
fn process_error_observer(
    aux: &mut AuxiliaryData,
    eo: &ErrorObserver,
    current_scope_id: ScopeId,
    current_observer_chain: &mut Vec<UserComponentId>,
) {
    const LIFECYCLE: Lifecycle = Lifecycle::Transient;

    let annotation_coordinates = AnnotationCoordinates {
        id: eo.coordinates.id.clone(),
        created_at: eo.coordinates.created_at.clone(),
    };
    let coordinates_id = aux
        .annotation_coordinates_interner
        .get_or_intern(annotation_coordinates);

    let component = UserComponent::ErrorObserver {
        source: coordinates_id,
    };
    let id = aux.intern_component(
        component,
        current_scope_id,
        LIFECYCLE,
        eo.registered_at.clone().into(),
    );
    current_observer_chain.push(id);
}

/// Process an error handler that has been
/// registered against the provided `Blueprint`.
/// It is associated with or nested under the provided `current_scope_id`.
fn process_error_handler(
    aux: &mut AuxiliaryData,
    eh: &pavex_bp_schema::ErrorHandler,
    current_scope_id: ScopeId,
) {
    const LIFECYCLE: Lifecycle = Lifecycle::Transient;

    let annotation_coordinates = AnnotationCoordinates {
        id: eh.coordinates.id.clone(),
        created_at: eh.coordinates.created_at.clone(),
    };
    let coordinates_id = aux
        .annotation_coordinates_interner
        .get_or_intern(annotation_coordinates);

    let component = UserComponent::ErrorHandler {
        source: coordinates_id.into(),
        target: ErrorHandlerTarget::ErrorType {
            error_ref_input_index: None,
        },
    };
    aux.intern_component(
        component,
        current_scope_id,
        LIFECYCLE,
        eh.registered_at.clone().into(),
    );
}

/// Process a prebuilt type that has been
/// registered against the provided `Blueprint`.
/// It is associated with or nested under the provided `current_scope_id`.
fn process_prebuilt_type(aux: &mut AuxiliaryData, si: &PrebuiltType, current_scope_id: ScopeId) {
    const LIFECYCLE: Lifecycle = Lifecycle::Singleton;

    let identifiers_id = aux
        .identifiers_interner
        .get_or_intern(si.input.type_.clone());
    let component = UserComponent::PrebuiltType {
        source: identifiers_id.into(),
    };
    let id = aux.intern_component(
        component,
        current_scope_id,
        LIFECYCLE,
        si.input.registered_at.clone().into(),
    );
    aux.id2cloning_strategy.insert(
        id,
        si.cloning_strategy.unwrap_or(CloningStrategy::NeverClone),
    );
}

/// Register a config type.
/// It is associated with or nested under the provided `current_scope_id`.
fn process_config_type(aux: &mut AuxiliaryData, t: &ConfigType, current_scope_id: ScopeId) {
    const LIFECYCLE: Lifecycle = Lifecycle::Singleton;

    let annotation_coordinates = AnnotationCoordinates {
        id: t.coordinates.id.clone(),
        created_at: t.coordinates.created_at.clone(),
    };
    let coordinates_id = aux
        .annotation_coordinates_interner
        .get_or_intern(annotation_coordinates);
    let component = UserComponent::ConfigType {
        key: String::new(), // Will be filled in during annotation resolution
        source: UserComponentSource::AnnotationCoordinates(coordinates_id),
    };
    let id = aux.intern_component(
        component,
        current_scope_id,
        LIFECYCLE,
        t.registered_at.clone().into(),
    );

    // If the user has specified/overriden the expected behaviour for the config type,
    // let's take note of it.
    // If not, we'll fill things in with the right default values later on,
    // when resolving annotation coordinates.
    if let Some(cloning_strategy) = t.cloning_strategy {
        aux.id2cloning_strategy.insert(id, cloning_strategy);
    }
    if let Some(default_if_missing) = t.default_if_missing {
        let default_strategy = if default_if_missing {
            DefaultStrategy::DefaultIfMissing
        } else {
            DefaultStrategy::Required
        };
        aux.config_id2default_strategy.insert(id, default_strategy);
    }
    if let Some(include_if_unused) = t.include_if_unused {
        aux.config_id2include_if_unused
            .insert(id, include_if_unused);
    }
}

/// Process the error handler registered against a (supposedly) fallible component, if
/// any.
fn process_legacy_component_specific_error_handler(
    aux: &mut AuxiliaryData,
    error_handler: &Option<Callable>,
    lifecycle: Lifecycle,
    scope_id: ScopeId,
    fallible_id: UserComponentId,
) {
    let Some(error_handler) = error_handler else {
        return;
    };
    let identifiers_id = aux
        .identifiers_interner
        .get_or_intern(error_handler.callable.clone());
    let component = UserComponent::ErrorHandler {
        source: identifiers_id.into(),
        target: ErrorHandlerTarget::FallibleComponent { fallible_id },
    };
    let error_handler_id = aux.intern_component(
        component,
        scope_id,
        lifecycle,
        error_handler.registered_at.clone().into(),
    );
    aux.fallible_id2error_handler_id
        .insert(fallible_id, error_handler_id);
}

/// Process the error handler registered against a (supposedly) fallible component, if
/// any.
fn process_component_specific_error_handler(
    aux: &mut AuxiliaryData,
    eh: &Option<ErrorHandler>,
    lifecycle: Lifecycle,
    scope_id: ScopeId,
    fallible_id: UserComponentId,
) {
    let Some(eh) = eh else {
        return;
    };
    let annotation_coordinates = AnnotationCoordinates {
        id: eh.coordinates.id.clone(),
        created_at: eh.coordinates.created_at.clone(),
    };
    let coordinates_id = aux
        .annotation_coordinates_interner
        .get_or_intern(annotation_coordinates);
    let component = UserComponent::ErrorHandler {
        source: coordinates_id.into(),
        target: ErrorHandlerTarget::FallibleComponent { fallible_id },
    };
    let error_handler_id = aux.intern_component(
        component,
        scope_id,
        lifecycle,
        eh.registered_at.clone().into(),
    );
    aux.fallible_id2error_handler_id
        .insert(fallible_id, error_handler_id);
}

/// Check the path of the registered route.
/// Emit diagnostics if the path is invalid—i.e. empty or missing a leading slash.
pub(super) fn validate_route_path(
    aux: &mut AuxiliaryData,
    route_id: UserComponentId,
    path: &str,
    diagnostics: &crate::diagnostic::DiagnosticSink,
) {
    // Empty paths are OK.
    if path.is_empty() {
        return;
    }
    if !path.starts_with('/') {
        diagnostics::route_path_must_start_with_a_slash(aux, path, route_id, diagnostics);
    }
}

/// Process the path prefix and the domain guard attached to this nested blueprint, if any.
/// Emit diagnostics if either is invalid—i.e. a prefix that's empty or missing a leading slash.
fn process_nesting_constraints(
    aux: &mut AuxiliaryData,
    nested_bp: &NestedBlueprint,
    diagnostics: &crate::diagnostic::DiagnosticSink,
) -> Result<(Option<String>, Option<DomainGuard>), ()> {
    let mut prefix = None;
    if let Some(path_prefix) = &nested_bp.path_prefix {
        let PathPrefix {
            path_prefix,
            registered_at: location,
        } = path_prefix;
        let mut has_errored = false;

        if path_prefix.is_empty() {
            diagnostics::path_prefix_cannot_be_empty(location, diagnostics);
            has_errored = true;
        }

        if !path_prefix.starts_with('/') {
            diagnostics::path_prefix_must_start_with_a_slash(path_prefix, location, diagnostics);
            has_errored = true;
        }

        if path_prefix.ends_with('/') {
            diagnostics::path_prefix_cannot_end_with_a_slash(path_prefix, location, diagnostics);
            has_errored = true;
        }

        if has_errored {
            return Err(());
        } else {
            prefix = Some(path_prefix.to_owned());
        }
    }

    let domain = if let Some(domain) = &nested_bp.domain {
        let Domain {
            domain,
            registered_at: location,
        } = domain;
        match DomainGuard::new(domain.into()) {
            Ok(guard) => {
                aux.domain_guard2locations
                    .entry(guard.clone())
                    .or_default()
                    .push(location.clone());
                Some(guard)
            }
            Err(e) => {
                diagnostics::invalid_domain_guard(location, e, diagnostics);
                return Err(());
            }
        }
    } else {
        None
    };
    Ok((prefix, domain))
}

mod diagnostics {
    use pavex_cli_diagnostic::CompilerDiagnostic;

    use crate::{
        compiler::analyses::domain::InvalidDomainConstraint,
        diagnostic::{
            self, DiagnosticSink, OptionalLabeledSpanExt, OptionalSourceSpanExt, TargetSpan,
        },
    };

    use super::*;

    pub(super) fn route_path_must_start_with_a_slash(
        aux: &AuxiliaryData,
        path: &str,
        route_id: UserComponentId,
        diagnostics: &DiagnosticSink,
    ) {
        let source = diagnostics.annotated(
            TargetSpan::RoutePath(&aux.id2registration[route_id]),
            "The path missing a leading '/'",
        );
        let err = anyhow::anyhow!(
            "Non-empty route paths must begin with a forward slash, `/`.\n`{path}` doesn't have one.",
        );
        let diagnostic = CompilerDiagnostic::builder(err)
            .optional_source(source)
            .help(format!("Use `/{path}` instead of `{path}`."));
        diagnostics.push(diagnostic.build());
    }

    pub(super) fn invalid_domain_guard(
        location: &Location,
        e: InvalidDomainConstraint,
        diagnostics: &DiagnosticSink,
    ) {
        let source = diagnostics.source(location).map(|s| {
            diagnostic::domain_span(s.source(), location)
                .labeled("The invalid domain".to_string())
                .attach(s)
        });
        let diagnostic = CompilerDiagnostic::builder(e).optional_source(source);
        diagnostics.push(diagnostic.build());
    }

    pub(super) fn path_prefix_cannot_be_empty(location: &Location, diagnostics: &DiagnosticSink) {
        let source = diagnostics.source(location).map(|s| {
            diagnostic::prefix_span(s.source(), location)
                .labeled("The empty prefix".to_string())
                .attach(s)
        });
        let err = anyhow::anyhow!("Path prefixes cannot be empty.");
        let diagnostic = CompilerDiagnostic::builder(err)
            .optional_source(source)
            .help(
                "If you don't want to add a common prefix to all routes in the nested blueprint, \
                    use the `nest` method directly."
                    .into(),
            );
        diagnostics.push(diagnostic.build());
    }

    pub(super) fn path_prefix_must_start_with_a_slash(
        prefix: &str,
        location: &Location,
        diagnostics: &DiagnosticSink,
    ) {
        let source = diagnostics.source(location).map(|s| {
            diagnostic::prefix_span(s.source(), location)
                .labeled("The prefix missing a leading '/'".to_string())
                .attach(s)
        });
        let err = anyhow::anyhow!(
            "Path prefixes must begin with a forward slash, `/`.\n\
                `{prefix}` doesn't.",
        );
        let diagnostic = CompilerDiagnostic::builder(err)
                .optional_source(source)
                .help(format!("Add a '/' at the beginning of the path prefix to fix this error: use `/{prefix}` instead of `{prefix}`."));
        diagnostics.push(diagnostic.build());
    }

    pub(super) fn path_prefix_cannot_end_with_a_slash(
        prefix: &str,
        location: &Location,
        diagnostics: &DiagnosticSink,
    ) {
        let source = diagnostics.source(location).map(|s| {
            diagnostic::prefix_span(s.source(), location)
                .labeled("The prefix ending with a trailing '/'".to_string())
                .attach(s)
        });
        let err = anyhow::anyhow!(
            "Path prefixes can't end with a trailing slash, `/`. \
                `{prefix}` does.\n\
                Trailing slashes in path prefixes increase the likelihood of having consecutive \
                slashes in the final route path, which is rarely desirable. If you want consecutive \
                slashes in the final route path, you can add them explicitly in the paths of the routes \
                registered against the nested blueprint.",
        );
        let correct_prefix = prefix.trim_end_matches('/');
        let diagnostic = CompilerDiagnostic::builder(err)
                .optional_source(source)
                .help(format!("Remove the '/' at the end of the path prefix to fix this error: use `{correct_prefix}` instead of `{prefix}`."));
        diagnostics.push(diagnostic.build());
    }
}
