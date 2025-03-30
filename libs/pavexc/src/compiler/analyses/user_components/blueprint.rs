use pavex_bp_schema::{
    Blueprint, Callable, CloningStrategy, Component, ConfigType, Constructor, CreatedAt, Domain,
    ErrorObserver, Fallback, Lifecycle, Location, NestedBlueprint, PathPrefix,
    PostProcessingMiddleware, PreProcessingMiddleware, PrebuiltType, RawIdentifiers, Route,
    WrappingMiddleware,
};

use super::UserComponentId;
use super::auxiliary::AuxiliaryData;
use crate::compiler::analyses::domain::DomainGuard;
use crate::compiler::analyses::user_components::router_key::RouterKey;
use crate::compiler::analyses::user_components::scope_graph::ScopeGraphBuilder;
use crate::compiler::analyses::user_components::{ScopeGraph, ScopeId, UserComponent};
use crate::compiler::component::DefaultStrategy;

/// A unique identifier for a `RawCallableIdentifiers`.
pub type RawIdentifierId = la_arena::Idx<RawIdentifiers>;

/// Process a [`Blueprint`], populating [`AuxiliaryData`] with all its registered components.
///
/// It returns a [`ScopeGraph`] too.
pub(super) fn process_blueprint(
    bp: &Blueprint,
    aux: &mut AuxiliaryData,
    diagnostics: &mut crate::diagnostic::DiagnosticSink,
) -> ScopeGraph {
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

    scope_graph_builder.build()
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
    diagnostics: &mut crate::diagnostic::DiagnosticSink,
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
            Component::PrebuiltType(si) => {
                process_prebuilt_type(aux, si, current_scope_id);
            }
            Component::ConfigType(t) => {
                process_config_type(aux, t, current_scope_id);
            }
            Component::Import(import) => {
                aux.imports.push((import.clone(), current_scope_id));
            }
        }
    }
    if let Some(fallback) = &fallback {
        process_fallback(
            aux,
            fallback,
            path_prefix,
            domain_guard,
            current_middleware_chain,
            current_observer_chain,
            current_scope_id,
            scope_graph_builder,
        );
    } else if is_root {
        // We need to have a top-level fallback handler.
        // If the user hasn't registered one against the top-level blueprint,
        // we must provide a framework default.
        let raw_callable_identifiers = RawIdentifiers::from_raw_parts(
            "pavex::router::default_fallback".to_owned(),
            CreatedAt {
                crate_name: "pavex".to_owned(),
                module_path: "pavex".to_owned(),
            },
        );
        let registered_fallback = Fallback {
            request_handler: Callable {
                callable: raw_callable_identifiers,
                // We don't have a location for the default fallback handler.
                // Nor do we have a way (yet) to identify this component as "framework provided".
                // Something to fix in the future.
                registered_at: bp.creation_location.clone(),
            },
            error_handler: None,
        };
        process_fallback(
            aux,
            &registered_fallback,
            path_prefix,
            domain_guard,
            current_middleware_chain,
            current_observer_chain,
            current_scope_id,
            scope_graph_builder,
        )
    }
}

/// Process a route that has been
/// registered against the provided `Blueprint`, including its error handler
/// (if present).
fn process_route(
    aux: &mut AuxiliaryData,
    registered_route: &Route,
    current_middleware_chain: &[UserComponentId],
    current_observer_chain: &[UserComponentId],
    current_scope_id: ScopeId,
    domain_guard: Option<DomainGuard>,
    path_prefix: Option<&str>,
    scope_graph_builder: &mut ScopeGraphBuilder,
    diagnostics: &mut crate::diagnostic::DiagnosticSink,
) {
    const ROUTE_LIFECYCLE: Lifecycle = Lifecycle::RequestScoped;

    let raw_callable_identifiers_id = aux
        .identifiers_interner
        .get_or_intern(registered_route.request_handler.callable.clone());
    let route_scope_id = scope_graph_builder.add_scope(current_scope_id, None);
    let router_key = {
        let path = match path_prefix {
            Some(prefix) => format!("{}{}", prefix, registered_route.path),
            None => registered_route.path.to_owned(),
        };
        RouterKey {
            path,
            domain_guard,
            method_guard: registered_route.method_guard.clone(),
        }
    };
    let component = UserComponent::RequestHandler {
        router_key,
        source: raw_callable_identifiers_id,
    };
    let request_handler_id = aux.intern_component(
        component,
        route_scope_id,
        ROUTE_LIFECYCLE,
        registered_route
            .request_handler
            .registered_at
            .clone()
            .into(),
    );

    aux.handler_id2middleware_ids
        .insert(request_handler_id, current_middleware_chain.to_owned());
    aux.handler_id2error_observer_ids
        .insert(request_handler_id, current_observer_chain.to_owned());

    validate_route(aux, request_handler_id, registered_route, diagnostics);

    process_error_handler(
        aux,
        &registered_route.error_handler,
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

    let raw_callable_identifiers_id = aux
        .identifiers_interner
        .get_or_intern(fallback.request_handler.callable.clone());
    let route_scope_id = scope_graph_builder.add_scope(current_scope_id, None);
    let component = UserComponent::Fallback {
        source: raw_callable_identifiers_id,
    };
    let fallback_id = aux.intern_component(
        component,
        route_scope_id,
        ROUTE_LIFECYCLE,
        fallback.request_handler.registered_at.clone().into(),
    );

    aux.handler_id2middleware_ids
        .insert(fallback_id, current_middleware_chain.to_owned());
    aux.handler_id2error_observer_ids
        .insert(fallback_id, current_observer_chain.to_owned());
    aux.fallback_id2path_prefix
        .insert(fallback_id, path_prefix.map(|s| s.to_owned()));
    aux.fallback_id2domain_guard
        .insert(fallback_id, domain_guard);

    process_error_handler(
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
    let identifiers_id = aux
        .identifiers_interner
        .get_or_intern(middleware.middleware.callable.clone());
    let component = UserComponent::WrappingMiddleware {
        source: identifiers_id,
    };
    let component_id = aux.intern_component(
        component,
        middleware_scope_id,
        MIDDLEWARE_LIFECYCLE,
        middleware.middleware.registered_at.clone().into(),
    );
    current_middleware_chain.push(component_id);

    process_error_handler(
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
    let identifiers_id = aux
        .identifiers_interner
        .get_or_intern(middleware.middleware.callable.clone());
    let component = UserComponent::PreProcessingMiddleware {
        source: identifiers_id,
    };
    let component_id = aux.intern_component(
        component,
        middleware_scope_id,
        MIDDLEWARE_LIFECYCLE,
        middleware.middleware.registered_at.clone().into(),
    );
    current_middleware_chain.push(component_id);

    process_error_handler(
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
    let identifiers_id = aux
        .identifiers_interner
        .get_or_intern(middleware.middleware.callable.clone());
    let component = UserComponent::PostProcessingMiddleware {
        source: identifiers_id,
    };
    let component_id = aux.intern_component(
        component,
        middleware_scope_id,
        MIDDLEWARE_LIFECYCLE,
        middleware.middleware.registered_at.clone().into(),
    );
    current_middleware_chain.push(component_id);

    process_error_handler(
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

    process_error_handler(
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

    let identifiers_id = aux
        .identifiers_interner
        .get_or_intern(eo.error_observer.callable.clone());
    let component = UserComponent::ErrorObserver {
        source: identifiers_id,
    };
    let id = aux.intern_component(
        component,
        current_scope_id,
        LIFECYCLE,
        eo.error_observer.registered_at.clone().into(),
    );
    current_observer_chain.push(id);
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
        source: identifiers_id,
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

    let identifiers_id = aux
        .identifiers_interner
        .get_or_intern(t.input.type_.clone());
    let component = UserComponent::ConfigType {
        key: t.key.clone(),
        source: identifiers_id,
    };
    let id = aux.intern_component(
        component,
        current_scope_id,
        LIFECYCLE,
        t.input.registered_at.clone().into(),
    );
    aux.id2cloning_strategy.insert(
        id,
        t.cloning_strategy
            .unwrap_or(CloningStrategy::CloneIfNecessary),
    );
    let default_strategy = t
        .default_if_missing
        .map(|b| {
            if b {
                DefaultStrategy::DefaultIfMissing
            } else {
                DefaultStrategy::Required
            }
        })
        .unwrap_or(DefaultStrategy::Required);
    aux.config_id2default_strategy.insert(id, default_strategy);
}

/// Process the error handler registered against a (supposedly) fallible component, if
/// any.
fn process_error_handler(
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
        fallible_id,
    };
    aux.intern_component(
        component,
        scope_id,
        lifecycle,
        error_handler.registered_at.clone().into(),
    );
}

/// Check the path of the registered route.
/// Emit diagnostics if the path is invalid—i.e. empty or missing a leading slash.
fn validate_route(
    aux: &mut AuxiliaryData,
    route_id: UserComponentId,
    route: &Route,
    diagnostics: &mut crate::diagnostic::DiagnosticSink,
) {
    // Empty paths are OK.
    if route.path.is_empty() {
        return;
    }
    if !route.path.starts_with('/') {
        diagnostics::route_path_must_start_with_a_slash(aux, route, route_id, diagnostics);
    }
}

/// Process the path prefix and the domain guard attached to this nested blueprint, if any.
/// Emit diagnostics if either is invalid—i.e. a prefix that's empty or missing a leading slash.
fn process_nesting_constraints(
    aux: &mut AuxiliaryData,
    nested_bp: &NestedBlueprint,
    diagnostics: &mut crate::diagnostic::DiagnosticSink,
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
        diagnostic::{self, OptionalLabeledSpanExt, OptionalSourceSpanExt, TargetSpan},
    };

    use super::*;

    pub(super) fn route_path_must_start_with_a_slash(
        aux: &AuxiliaryData,
        route: &Route,
        route_id: UserComponentId,
        diagnostics: &mut crate::diagnostic::DiagnosticSink,
    ) {
        let source = diagnostics.annotated(
            TargetSpan::RoutePath(&aux.id2registration[&route_id]),
            "The path missing a leading '/'",
        );
        let path = &route.path;
        let err = anyhow::anyhow!(
            "Route paths must either be empty or begin with a forward slash, `/`.\n`{path}` is not empty and it doesn't begin with a `/`.",
        );
        let diagnostic = CompilerDiagnostic::builder(err)
                .optional_source(source)
                .help(format!("Add a '/' at the beginning of the route path to fix this error: use `/{path}` instead of `{path}`."));
        diagnostics.push(diagnostic.build());
    }

    pub(super) fn invalid_domain_guard(
        location: &Location,
        e: InvalidDomainConstraint,
        diagnostics: &mut crate::diagnostic::DiagnosticSink,
    ) {
        let source = diagnostics.source(location).map(|s| {
            diagnostic::domain_span(s.source(), location)
                .labeled("The invalid domain".to_string())
                .attach(s)
        });
        let diagnostic = CompilerDiagnostic::builder(e).optional_source(source);
        diagnostics.push(diagnostic.build());
    }

    pub(super) fn path_prefix_cannot_be_empty(
        location: &Location,
        diagnostics: &mut crate::diagnostic::DiagnosticSink,
    ) {
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
        diagnostics: &mut crate::diagnostic::DiagnosticSink,
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
        diagnostics: &mut crate::diagnostic::DiagnosticSink,
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
                slashes in the final route path, which is rarely desireable. If you want consecutive \
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
