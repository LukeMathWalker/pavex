use ahash::{HashMap, HashMapExt};
use anyhow::anyhow;
use guppy::graph::PackageGraph;
use indexmap::IndexMap;
use std::collections::BTreeMap;

use pavex_bp_schema::{
    Blueprint, Callable, CloningStrategy, Component, ConfigType, Constructor, Domain,
    ErrorObserver, Fallback, Lifecycle, Lint, LintSetting, Location, NestedBlueprint, PathPrefix,
    PostProcessingMiddleware, PreProcessingMiddleware, PrebuiltType, RawIdentifiers, RegisteredAt,
    Route, WrappingMiddleware,
};

use crate::compiler::analyses::domain::{DomainGuard, InvalidDomainConstraint};
use crate::compiler::analyses::user_components::router_key::RouterKey;
use crate::compiler::analyses::user_components::scope_graph::ScopeGraphBuilder;
use crate::compiler::analyses::user_components::{ScopeGraph, ScopeId};
use crate::compiler::component::DefaultStrategy;
use crate::compiler::interner::Interner;
use crate::diagnostic::{CompilerDiagnostic, ComponentKind, OptionalSourceSpanExt};
use crate::{diagnostic, try_source};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
/// A component registered by a framework user against the `Blueprint` for their application.
///
/// All user components can be directly mapped back to the source code that registered them.
///
/// See [`UserComponentDb`] for more details.
///
/// [`UserComponentDb`]: crate::compiler::analyses::user_components::UserComponentDb
pub enum UserComponent {
    RequestHandler {
        raw_callable_identifiers_id: RawIdentifierId,
        router_key: RouterKey,
        scope_id: ScopeId,
    },
    Fallback {
        raw_callable_identifiers_id: RawIdentifierId,
        scope_id: ScopeId,
    },
    ErrorHandler {
        raw_callable_identifiers_id: RawIdentifierId,
        fallible_callable_identifiers_id: UserComponentId,
        scope_id: ScopeId,
    },
    Constructor {
        raw_callable_identifiers_id: RawIdentifierId,
        scope_id: ScopeId,
    },
    PrebuiltType {
        raw_identifiers_id: RawIdentifierId,
        scope_id: ScopeId,
    },
    ConfigType {
        raw_identifiers_id: RawIdentifierId,
        key: String,
        scope_id: ScopeId,
    },
    WrappingMiddleware {
        raw_callable_identifiers_id: RawIdentifierId,
        scope_id: ScopeId,
    },
    PostProcessingMiddleware {
        raw_callable_identifiers_id: RawIdentifierId,
        scope_id: ScopeId,
    },
    PreProcessingMiddleware {
        raw_callable_identifiers_id: RawIdentifierId,
        scope_id: ScopeId,
    },
    ErrorObserver {
        raw_callable_identifiers_id: RawIdentifierId,
        scope_id: ScopeId,
    },
}

impl UserComponent {
    /// Returns the tag for the "variant" of this `UserComponent`.
    ///
    /// Useful when you don't need to access the actual data attached component.
    pub fn kind(&self) -> ComponentKind {
        match self {
            UserComponent::RequestHandler { .. } => ComponentKind::RequestHandler,
            UserComponent::ErrorHandler { .. } => ComponentKind::ErrorHandler,
            UserComponent::Constructor { .. } => ComponentKind::Constructor,
            UserComponent::PrebuiltType { .. } => ComponentKind::PrebuiltType,
            UserComponent::ConfigType { .. } => ComponentKind::ConfigType,
            UserComponent::WrappingMiddleware { .. } => ComponentKind::WrappingMiddleware,
            UserComponent::Fallback { .. } => ComponentKind::RequestHandler,
            UserComponent::ErrorObserver { .. } => ComponentKind::ErrorObserver,
            UserComponent::PostProcessingMiddleware { .. } => {
                ComponentKind::PostProcessingMiddleware
            }
            UserComponent::PreProcessingMiddleware { .. } => ComponentKind::PreProcessingMiddleware,
        }
    }

    /// Returns an id that points at the raw identifiers for the callable that
    /// this [`UserComponent`] is associated with.
    pub fn raw_identifiers_id(&self) -> RawIdentifierId {
        match self {
            UserComponent::PostProcessingMiddleware {
                raw_callable_identifiers_id,
                ..
            }
            | UserComponent::PreProcessingMiddleware {
                raw_callable_identifiers_id,
                ..
            }
            | UserComponent::PrebuiltType {
                raw_identifiers_id: raw_callable_identifiers_id,
                ..
            }
            | UserComponent::ConfigType {
                raw_identifiers_id: raw_callable_identifiers_id,
                ..
            }
            | UserComponent::WrappingMiddleware {
                raw_callable_identifiers_id,
                ..
            }
            | UserComponent::Fallback {
                raw_callable_identifiers_id,
                ..
            }
            | UserComponent::RequestHandler {
                raw_callable_identifiers_id,
                ..
            }
            | UserComponent::ErrorHandler {
                raw_callable_identifiers_id,
                ..
            }
            | UserComponent::ErrorObserver {
                raw_callable_identifiers_id,
                ..
            }
            | UserComponent::Constructor {
                raw_callable_identifiers_id,
                ..
            } => *raw_callable_identifiers_id,
        }
    }

    /// Returns the [`ScopeId`] for the scope that this [`UserComponent`] is associated with.
    pub fn scope_id(&self) -> ScopeId {
        match self {
            UserComponent::ErrorObserver { scope_id, .. }
            | UserComponent::RequestHandler { scope_id, .. }
            | UserComponent::Fallback { scope_id, .. }
            | UserComponent::ErrorHandler { scope_id, .. }
            | UserComponent::WrappingMiddleware { scope_id, .. }
            | UserComponent::PostProcessingMiddleware { scope_id, .. }
            | UserComponent::PrebuiltType { scope_id, .. }
            | UserComponent::ConfigType { scope_id, .. }
            | UserComponent::PreProcessingMiddleware { scope_id, .. }
            | UserComponent::Constructor { scope_id, .. } => *scope_id,
        }
    }

    /// Returns the raw identifiers for the callable that this `UserComponent` is associated with.
    pub(super) fn raw_identifiers<'b>(&self, db: &'b RawUserComponentDb) -> &'b RawIdentifiers {
        &db.identifiers_interner[self.raw_identifiers_id()]
    }
}

/// A unique identifier for a `RawCallableIdentifiers`.
pub type RawIdentifierId = la_arena::Idx<RawIdentifiers>;

/// A unique identifier for a [`UserComponent`].
pub type UserComponentId = la_arena::Idx<UserComponent>;

/// A database that contains all the user components that have been registered against the
/// `Blueprint` for the application.
///
/// For each component, we keep track of:
/// - the raw identifiers for the callable that it is associated with;
/// - the source code location where it was registered (for error reporting purposes);
/// - the lifecycle of the component;
/// - the cloning strategy of the component (if it is a constructor);
/// - the scope that the component belongs to.
///
/// We call them "raw" components because we are yet to resolve the paths to the actual
/// functions that they refer to and perform higher-level checks (e.g. does a constructor
/// return a type or unit?).
pub(super) struct RawUserComponentDb {
    pub(super) component_interner: Interner<UserComponent>,
    pub(super) identifiers_interner: Interner<RawIdentifiers>,
    /// Associate each user-registered component with the location it was
    /// registered at against the `Blueprint` in the user's source code.
    ///
    /// Invariants: there is an entry for every single user component.
    pub(super) id2locations: HashMap<UserComponentId, Location>,
    /// Associate each user-registered component with its lifecycle.
    ///
    /// Invariants: there is an entry for every single user component.
    pub(super) id2lifecycle: HashMap<UserComponentId, Lifecycle>,
    /// Associate each user-registered component with its lint overrides, if any.
    /// If there is no entry for a component, there are no overrides.
    pub(super) id2lints: HashMap<UserComponentId, BTreeMap<Lint, LintSetting>>,
    /// Determine if a type can be cloned or not.
    ///
    /// Invariants: there is an entry for every constructor, configuration type and prebuilt type.
    pub(super) id2cloning_strategy: HashMap<UserComponentId, CloningStrategy>,
    /// Determine if a configuration type should have a default.
    ///
    /// Invariants: there is an entry for configuration type.
    pub(super) config_id2default_strategy: HashMap<UserComponentId, DefaultStrategy>,
    /// Associate each request handler with the ordered list of middlewares that wrap around it.
    ///
    /// Invariants: there is an entry for every single request handler.
    pub(super) handler_id2middleware_ids: HashMap<UserComponentId, Vec<UserComponentId>>,
    /// Associate each request handler with the ordered list of error observers
    /// that must be invoked if there is an error while handling a request.
    ///
    /// Invariants: there is an entry for every single request handler.
    pub(super) handler_id2error_observer_ids: HashMap<UserComponentId, Vec<UserComponentId>>,
    /// Associate each user-registered fallback with the path prefix of the `Blueprint`
    /// it was registered against.
    /// If it was registered against a deeply nested `Blueprint`, it contains the **concatenated**
    /// path prefixes of all the `Blueprint`s that it was nested under.
    ///
    /// Invariants: there is an entry for every single fallback.
    pub(super) fallback_id2path_prefix: HashMap<UserComponentId, Option<String>>,
    /// Associate each user-registered fallback with the domain guard of the `Blueprint`
    /// it was registered against, if any.
    /// If it was registered against a deeply nested `Blueprint`, it contains the domain guard
    /// of the **innermost** `Blueprint` with a non-empty domain guard that it was nested under.
    ///
    /// Invariants: there is an entry for every single fallback.
    pub(super) fallback_id2domain_guard: HashMap<UserComponentId, Option<DomainGuard>>,
    /// Associate each domain guard with the location it was registered at against the `Blueprint`.
    ///
    /// The same guard can be registered at multiple locations, so we use a `Vec` to store them.
    pub(super) domain_guard2locations: IndexMap<DomainGuard, Vec<Location>>,
}

/// Used in [`RawUserComponentDb::build`] to keep track of the nested blueprints that we still
/// need to process.
struct QueueItem<'a> {
    parent_scope_id: ScopeId,
    parent_path_prefix: Option<String>,
    parent_domain_guard: Option<DomainGuard>,
    nested_bp: &'a NestedBlueprint,
    current_middleware_chain: Vec<UserComponentId>,
    current_observer_chain: Vec<UserComponentId>,
}

// The public `build` method alongside its private supporting routines.
impl RawUserComponentDb {
    /// Process a `Blueprint` and return a `UserComponentDb` that contains all the user components
    /// that have been registered against it.
    pub fn build(
        bp: &Blueprint,
        package_graph: &PackageGraph,
        diagnostics: &mut Vec<miette::Error>,
    ) -> (Self, ScopeGraph) {
        let mut self_ = Self {
            component_interner: Interner::new(),
            identifiers_interner: Interner::new(),
            id2locations: HashMap::new(),
            id2lifecycle: HashMap::new(),
            id2lints: HashMap::new(),
            id2cloning_strategy: HashMap::new(),
            config_id2default_strategy: HashMap::new(),
            handler_id2middleware_ids: HashMap::new(),
            handler_id2error_observer_ids: HashMap::new(),
            fallback_id2path_prefix: HashMap::new(),
            fallback_id2domain_guard: HashMap::new(),
            domain_guard2locations: IndexMap::new(),
        };
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

        Self::process_blueprint(
            &mut self_,
            bp,
            root_scope_id,
            None,
            None,
            &mut scope_graph_builder,
            &mut current_middleware_chain,
            &mut current_observer_chain,
            true,
            &mut processing_queue,
            package_graph,
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
            let nested_scope_id = scope_graph_builder
                .add_scope(parent_scope_id, Some(nested_bp.nesting_location.clone()));
            let Ok((current_prefix, current_domain)) =
                self_.process_nesting_constraints(nested_bp, package_graph, diagnostics)
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

            Self::process_blueprint(
                &mut self_,
                &nested_bp.blueprint,
                nested_scope_id,
                domain_guard,
                path_prefix.as_deref(),
                &mut scope_graph_builder,
                &mut current_middleware_chain,
                &mut current_observer_chain,
                false,
                &mut processing_queue,
                package_graph,
                diagnostics,
            );
        }

        #[cfg(debug_assertions)]
        self_.check_invariants();

        let scope_graph = scope_graph_builder.build();
        (self_, scope_graph)
    }

    /// Register with [`RawUserComponentDb`] all the user components that have been
    /// registered against the provided `Blueprint`.
    /// All components are associated with or nested under the provided `current_scope_id`.
    ///
    /// If `path_prefix` is `Some`, then it is prepended to the path of each route
    /// in `Blueprint`.
    fn process_blueprint<'a>(
        &mut self,
        bp: &'a Blueprint,
        current_scope_id: ScopeId,
        domain_guard: Option<DomainGuard>,
        path_prefix: Option<&str>,
        scope_graph_builder: &mut ScopeGraphBuilder,
        current_middleware_chain: &mut Vec<UserComponentId>,
        current_observer_chain: &mut Vec<UserComponentId>,
        is_root: bool,
        bp_queue: &mut Vec<QueueItem<'a>>,
        package_graph: &PackageGraph,
        diagnostics: &mut Vec<miette::Error>,
    ) {
        let mut fallback: Option<&Fallback> = None;
        for component in &bp.components {
            match component {
                Component::Constructor(c) => {
                    self.process_constructor(c, current_scope_id);
                }
                Component::WrappingMiddleware(w) => {
                    self.process_middleware(
                        w,
                        current_scope_id,
                        current_middleware_chain,
                        scope_graph_builder,
                    );
                }
                Component::PreProcessingMiddleware(p) => {
                    self.process_pre_processing_middleware(
                        p,
                        current_scope_id,
                        current_middleware_chain,
                        scope_graph_builder,
                    );
                }
                Component::PostProcessingMiddleware(p) => {
                    self.process_post_processing_middleware(
                        p,
                        current_scope_id,
                        current_middleware_chain,
                        scope_graph_builder,
                    );
                }
                Component::Route(r) => self.process_route(
                    r,
                    current_middleware_chain,
                    current_observer_chain,
                    current_scope_id,
                    domain_guard.clone(),
                    path_prefix,
                    scope_graph_builder,
                    package_graph,
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
                    self.process_error_observer(eo, current_scope_id, current_observer_chain);
                }
                Component::PrebuiltType(si) => {
                    self.process_prebuilt_type(si, current_scope_id);
                }
                Component::ConfigType(t) => {
                    self.process_config_type(t, current_scope_id);
                }
            }
        }
        if let Some(fallback) = &fallback {
            self.process_fallback(
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
                RegisteredAt {
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
                    location: bp.creation_location.clone(),
                },
                error_handler: None,
            };
            self.process_fallback(
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

    /// Register with [`RawUserComponentDb`] a route that has been
    /// registered against the provided `Blueprint`, including its error handler
    /// (if present).
    fn process_route(
        &mut self,
        registered_route: &Route,
        current_middleware_chain: &[UserComponentId],
        current_observer_chain: &[UserComponentId],
        current_scope_id: ScopeId,
        domain_guard: Option<DomainGuard>,
        path_prefix: Option<&str>,
        scope_graph_builder: &mut ScopeGraphBuilder,
        package_graph: &PackageGraph,
        diagnostics: &mut Vec<miette::Error>,
    ) {
        const ROUTE_LIFECYCLE: Lifecycle = Lifecycle::RequestScoped;

        let raw_callable_identifiers_id = self
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
            raw_callable_identifiers_id,
            router_key,
            scope_id: route_scope_id,
        };
        let request_handler_id = self.intern_component(
            component,
            ROUTE_LIFECYCLE,
            registered_route.request_handler.location.to_owned(),
        );

        self.handler_id2middleware_ids
            .insert(request_handler_id, current_middleware_chain.to_owned());
        self.handler_id2error_observer_ids
            .insert(request_handler_id, current_observer_chain.to_owned());

        self.validate_route(
            request_handler_id,
            registered_route,
            package_graph,
            diagnostics,
        );

        self.process_error_handler(
            &registered_route.error_handler,
            ROUTE_LIFECYCLE,
            current_scope_id,
            request_handler_id,
        );
    }

    /// Register with [`RawUserComponentDb`] the fallback that has been
    /// registered against the provided `Blueprint`, including its error handler
    /// (if present).
    fn process_fallback(
        &mut self,
        fallback: &Fallback,
        path_prefix: Option<&str>,
        domain_guard: Option<DomainGuard>,
        current_middleware_chain: &[UserComponentId],
        current_observer_chain: &[UserComponentId],
        current_scope_id: ScopeId,
        scope_graph_builder: &mut ScopeGraphBuilder,
    ) {
        const ROUTE_LIFECYCLE: Lifecycle = Lifecycle::RequestScoped;

        let raw_callable_identifiers_id = self
            .identifiers_interner
            .get_or_intern(fallback.request_handler.callable.clone());
        let route_scope_id = scope_graph_builder.add_scope(current_scope_id, None);
        let component = UserComponent::Fallback {
            raw_callable_identifiers_id,
            scope_id: route_scope_id,
        };
        let fallback_id = self.intern_component(
            component,
            ROUTE_LIFECYCLE,
            fallback.request_handler.location.to_owned(),
        );

        self.handler_id2middleware_ids
            .insert(fallback_id, current_middleware_chain.to_owned());
        self.handler_id2error_observer_ids
            .insert(fallback_id, current_observer_chain.to_owned());
        self.fallback_id2path_prefix
            .insert(fallback_id, path_prefix.map(|s| s.to_owned()));
        self.fallback_id2domain_guard
            .insert(fallback_id, domain_guard);

        self.process_error_handler(
            &fallback.error_handler,
            ROUTE_LIFECYCLE,
            current_scope_id,
            fallback_id,
        );
    }

    /// Register with [`RawUserComponentDb`] a middleware that has been
    /// registered against the provided `Blueprint`, including its error handler
    /// (if present).
    fn process_middleware(
        &mut self,
        middleware: &WrappingMiddleware,
        current_scope_id: ScopeId,
        current_middleware_chain: &mut Vec<UserComponentId>,
        scope_graph_builder: &mut ScopeGraphBuilder,
    ) {
        const MIDDLEWARE_LIFECYCLE: Lifecycle = Lifecycle::RequestScoped;

        let middleware_scope_id = scope_graph_builder.add_scope(current_scope_id, None);
        let raw_callable_identifiers_id = self
            .identifiers_interner
            .get_or_intern(middleware.middleware.callable.clone());
        let component = UserComponent::WrappingMiddleware {
            raw_callable_identifiers_id,
            scope_id: middleware_scope_id,
        };
        let component_id = self.intern_component(
            component,
            MIDDLEWARE_LIFECYCLE,
            middleware.middleware.location.clone(),
        );
        current_middleware_chain.push(component_id);

        self.process_error_handler(
            &middleware.error_handler,
            MIDDLEWARE_LIFECYCLE,
            current_scope_id,
            component_id,
        );
    }

    /// Register with [`RawUserComponentDb`] a pre-processing middleware that has been
    /// registered against the provided `Blueprint`, including its error handler
    /// (if present).
    fn process_pre_processing_middleware(
        &mut self,
        middleware: &PreProcessingMiddleware,
        current_scope_id: ScopeId,
        current_middleware_chain: &mut Vec<UserComponentId>,
        scope_graph_builder: &mut ScopeGraphBuilder,
    ) {
        const MIDDLEWARE_LIFECYCLE: Lifecycle = Lifecycle::RequestScoped;

        let middleware_scope_id = scope_graph_builder.add_scope(current_scope_id, None);
        let raw_callable_identifiers_id = self
            .identifiers_interner
            .get_or_intern(middleware.middleware.callable.clone());
        let component = UserComponent::PreProcessingMiddleware {
            raw_callable_identifiers_id,
            scope_id: middleware_scope_id,
        };
        let component_id = self.intern_component(
            component,
            MIDDLEWARE_LIFECYCLE,
            middleware.middleware.location.clone(),
        );
        current_middleware_chain.push(component_id);

        self.process_error_handler(
            &middleware.error_handler,
            MIDDLEWARE_LIFECYCLE,
            current_scope_id,
            component_id,
        );
    }

    /// Register with [`RawUserComponentDb`] a post-processing middleware that has been
    /// registered against the provided `Blueprint`, including its error handler
    /// (if present).
    fn process_post_processing_middleware(
        &mut self,
        middleware: &PostProcessingMiddleware,
        current_scope_id: ScopeId,
        current_middleware_chain: &mut Vec<UserComponentId>,
        scope_graph_builder: &mut ScopeGraphBuilder,
    ) {
        const MIDDLEWARE_LIFECYCLE: Lifecycle = Lifecycle::RequestScoped;

        let middleware_scope_id = scope_graph_builder.add_scope(current_scope_id, None);
        let raw_callable_identifiers_id = self
            .identifiers_interner
            .get_or_intern(middleware.middleware.callable.clone());
        let component = UserComponent::PostProcessingMiddleware {
            raw_callable_identifiers_id,
            scope_id: middleware_scope_id,
        };
        let component_id = self.intern_component(
            component,
            MIDDLEWARE_LIFECYCLE,
            middleware.middleware.location.clone(),
        );
        current_middleware_chain.push(component_id);

        self.process_error_handler(
            &middleware.error_handler,
            MIDDLEWARE_LIFECYCLE,
            current_scope_id,
            component_id,
        );
    }

    /// Register with [`RawUserComponentDb`] a constructor that has been
    /// registered against the provided `Blueprint`, including its error handler
    /// (if present).
    /// It is associated with or nested under the provided `current_scope_id`.
    fn process_constructor(&mut self, constructor: &Constructor, current_scope_id: ScopeId) {
        let raw_callable_identifiers_id = self
            .identifiers_interner
            .get_or_intern(constructor.constructor.callable.clone());
        let component = UserComponent::Constructor {
            raw_callable_identifiers_id,
            scope_id: current_scope_id,
        };
        let lifecycle = constructor.lifecycle;
        let constructor_id = self.intern_component(
            component,
            lifecycle,
            constructor.constructor.location.clone(),
        );
        self.id2cloning_strategy.insert(
            constructor_id,
            constructor
                .cloning_strategy
                .unwrap_or(CloningStrategy::NeverClone),
        );
        if !constructor.lints.is_empty() {
            self.id2lints
                .insert(constructor_id, constructor.lints.clone());
        }

        self.process_error_handler(
            &constructor.error_handler,
            lifecycle,
            current_scope_id,
            constructor_id,
        );
    }

    /// Register with [`RawUserComponentDb`] an error observer that has been
    /// registered against the provided `Blueprint`.
    /// It is associated with or nested under the provided `current_scope_id`.
    fn process_error_observer(
        &mut self,
        eo: &ErrorObserver,
        current_scope_id: ScopeId,
        current_observer_chain: &mut Vec<UserComponentId>,
    ) {
        const LIFECYCLE: Lifecycle = Lifecycle::Transient;

        let raw_callable_identifiers_id = self
            .identifiers_interner
            .get_or_intern(eo.error_observer.callable.clone());
        let component = UserComponent::ErrorObserver {
            raw_callable_identifiers_id,
            scope_id: current_scope_id,
        };
        let id = self.intern_component(component, LIFECYCLE, eo.error_observer.location.clone());
        current_observer_chain.push(id);
    }

    /// Register with [`RawUserComponentDb`] a prebuilt type that has been
    /// registered against the provided `Blueprint`.
    /// It is associated with or nested under the provided `current_scope_id`.
    fn process_prebuilt_type(&mut self, si: &PrebuiltType, current_scope_id: ScopeId) {
        const LIFECYCLE: Lifecycle = Lifecycle::Singleton;

        let raw_callable_identifiers_id = self
            .identifiers_interner
            .get_or_intern(si.input.type_.clone());
        let component = UserComponent::PrebuiltType {
            raw_identifiers_id: raw_callable_identifiers_id,
            scope_id: current_scope_id,
        };
        let id = self.intern_component(component, LIFECYCLE, si.input.location.clone());
        self.id2cloning_strategy.insert(
            id,
            si.cloning_strategy.unwrap_or(CloningStrategy::NeverClone),
        );
    }

    /// Register a config type against [`RawUserComponentDb`].
    /// It is associated with or nested under the provided `current_scope_id`.
    fn process_config_type(&mut self, t: &ConfigType, current_scope_id: ScopeId) {
        const LIFECYCLE: Lifecycle = Lifecycle::Singleton;

        let raw_callable_identifiers_id = self
            .identifiers_interner
            .get_or_intern(t.input.type_.clone());
        let component = UserComponent::ConfigType {
            raw_identifiers_id: raw_callable_identifiers_id,
            key: t.key.clone(),
            scope_id: current_scope_id,
        };
        let id = self.intern_component(component, LIFECYCLE, t.input.location.clone());
        self.id2cloning_strategy.insert(
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
        self.config_id2default_strategy.insert(id, default_strategy);
    }

    /// A helper function to intern a component without forgetting to do the necessary
    /// bookeeping for the metadata (location and lifecycle) that are common to all
    /// components.
    fn intern_component(
        &mut self,
        component: UserComponent,
        lifecycle: Lifecycle,
        location: Location,
    ) -> UserComponentId {
        let component_id = self.component_interner.get_or_intern(component);
        self.id2lifecycle.insert(component_id, lifecycle);
        self.id2locations.insert(component_id, location);
        component_id
    }

    /// Process the error handler registered against a (supposedly) fallible component, if
    /// any.
    fn process_error_handler(
        &mut self,
        error_handler: &Option<Callable>,
        lifecycle: Lifecycle,
        scope_id: ScopeId,
        fallible_component_id: UserComponentId,
    ) {
        let Some(error_handler) = error_handler else {
            return;
        };
        let raw_callable_identifiers_id = self
            .identifiers_interner
            .get_or_intern(error_handler.callable.clone());
        let component = UserComponent::ErrorHandler {
            raw_callable_identifiers_id,
            fallible_callable_identifiers_id: fallible_component_id,
            scope_id,
        };
        self.intern_component(component, lifecycle, error_handler.location.to_owned());
    }

    /// Check the path of the registered route.
    /// Emit diagnostics if the path is invalid—i.e. empty or missing a leading slash.
    fn validate_route(
        &self,
        route_id: UserComponentId,
        route: &Route,
        package_graph: &PackageGraph,
        diagnostics: &mut Vec<miette::Error>,
    ) {
        // Empty paths are OK.
        if route.path.is_empty() {
            return;
        }
        if !route.path.starts_with('/') {
            self.route_path_must_start_with_a_slash(route, route_id, package_graph, diagnostics);
        }
    }

    /// Process the path prefix and the domain guard attached to this nested blueprint, if any.
    /// Emit diagnostics if either is invalid—i.e. a prefix that's empty or missing a leading slash.
    fn process_nesting_constraints(
        &mut self,
        nested_bp: &NestedBlueprint,
        package_graph: &PackageGraph,
        diagnostics: &mut Vec<miette::Error>,
    ) -> Result<(Option<String>, Option<DomainGuard>), ()> {
        let mut prefix = None;
        if let Some(path_prefix) = &nested_bp.path_prefix {
            let PathPrefix {
                path_prefix,
                location,
            } = path_prefix;
            let mut has_errored = false;

            if path_prefix.is_empty() {
                self.path_prefix_cannot_be_empty(location, package_graph, diagnostics);
                has_errored = true;
            }

            if !path_prefix.starts_with('/') {
                self.path_prefix_must_start_with_a_slash(
                    path_prefix,
                    location,
                    package_graph,
                    diagnostics,
                );
                has_errored = true;
            }

            if path_prefix.ends_with('/') {
                self.path_prefix_cannot_end_with_a_slash(
                    path_prefix,
                    location,
                    package_graph,
                    diagnostics,
                );
                has_errored = true;
            }

            if has_errored {
                return Err(());
            } else {
                prefix = Some(path_prefix.to_owned());
            }
        }

        let domain = if let Some(domain) = &nested_bp.domain {
            let Domain { domain, location } = domain;
            match DomainGuard::new(domain.into()) {
                Ok(guard) => {
                    self.domain_guard2locations
                        .entry(guard.clone())
                        .or_default()
                        .push(location.clone());
                    Some(guard)
                }
                Err(e) => {
                    self.invalid_domain_guard(location, e, package_graph, diagnostics);
                    return Err(());
                }
            }
        } else {
            None
        };
        Ok((prefix, domain))
    }

    /// Validate that all internal invariants are satisfied.
    #[cfg(debug_assertions)]
    fn check_invariants(&self) {
        for (id, component) in self.iter() {
            assert!(
                self.id2lifecycle.contains_key(&id),
                "There is no lifecycle registered for the user-provided {} #{id:?}",
                component.kind()
            );
            assert!(
                self.id2locations.contains_key(&id),
                "There is no location registered for the user-provided {} #{id:?}",
                component.kind()
            );
            match component {
                UserComponent::Constructor { .. } | UserComponent::PrebuiltType { .. } => {
                    assert!(
                        self.id2cloning_strategy.contains_key(&id),
                        "There is no cloning strategy registered for the user-registered {} #{id:?}",
                        component.kind(),
                    );
                }
                UserComponent::ConfigType { .. } => {
                    assert!(
                        self.id2cloning_strategy.contains_key(&id),
                        "There is no cloning strategy registered for the user-registered {} #{id:?}",
                        component.kind(),
                    );
                    assert!(
                        self.config_id2default_strategy.contains_key(&id),
                        "There is no default strategy registered for the user-registered {} #{id:?}",
                        component.kind(),
                    );
                }
                UserComponent::RequestHandler { .. } => {
                    assert!(
                        self.handler_id2middleware_ids.contains_key(&id),
                        "The middleware chain is missing for the user-registered request handler #{:?}",
                        id
                    );
                    assert!(
                        self.handler_id2error_observer_ids.contains_key(&id),
                        "The list of error observers is missing for the user-registered request handler #{:?}",
                        id
                    );
                }
                UserComponent::Fallback { .. } => {
                    assert!(
                        self.handler_id2middleware_ids.contains_key(&id),
                        "The middleware chain is missing for the user-registered fallback #{:?}",
                        id
                    );
                    assert!(
                        self.handler_id2error_observer_ids.contains_key(&id),
                        "The list of error observers is missing for the user-registered fallback #{:?}",
                        id
                    );
                    assert!(
                        self.fallback_id2path_prefix.contains_key(&id),
                        "There is no path prefix associated with the user-registered fallback #{:?}",
                        id
                    );
                    assert!(
                        self.fallback_id2domain_guard.contains_key(&id),
                        "There is no domain guard associated with the user-registered fallback #{:?}",
                        id
                    );
                }
                UserComponent::ErrorHandler { .. }
                | UserComponent::WrappingMiddleware { .. }
                | UserComponent::PostProcessingMiddleware { .. }
                | UserComponent::PreProcessingMiddleware { .. }
                | UserComponent::ErrorObserver { .. } => {}
            }
        }
    }
}

impl RawUserComponentDb {
    /// Iterate over all the user components in the database, returning their id and the associated
    /// `UserComponent`.
    pub fn iter(
        &self,
    ) -> impl ExactSizeIterator<Item = (UserComponentId, &UserComponent)> + DoubleEndedIterator
    {
        self.component_interner.iter()
    }

    pub fn components(&self) -> impl Iterator<Item = &UserComponent> {
        self.component_interner.values()
    }

    /// Return the location where the component with the given id was registered against the
    /// application blueprint.
    pub fn get_location(&self, id: UserComponentId) -> &Location {
        &self.id2locations[&id]
    }
}

impl std::ops::Index<UserComponentId> for RawUserComponentDb {
    type Output = UserComponent;

    fn index(&self, index: UserComponentId) -> &Self::Output {
        &self.component_interner[index]
    }
}

impl std::ops::Index<&UserComponentId> for RawUserComponentDb {
    type Output = UserComponent;

    fn index(&self, index: &UserComponentId) -> &Self::Output {
        &self.component_interner[*index]
    }
}

impl std::ops::Index<&UserComponent> for RawUserComponentDb {
    type Output = UserComponentId;

    fn index(&self, index: &UserComponent) -> &Self::Output {
        &self.component_interner[index]
    }
}

/// All diagnostic-related code.
impl RawUserComponentDb {
    fn route_path_must_start_with_a_slash(
        &self,
        route: &Route,
        route_id: UserComponentId,
        package_graph: &PackageGraph,
        diagnostics: &mut Vec<miette::Error>,
    ) {
        let location = self.get_location(route_id);
        let source = try_source!(location, package_graph, diagnostics);
        let label = source.as_ref().and_then(|source| {
            diagnostic::get_route_path_span(source, location)
                .labeled("The path missing a leading '/'".to_string())
        });
        let path = &route.path;
        let err = anyhow!(
            "Route paths must either be empty or begin with a forward slash, `/`.\n`{path}` is not empty and it doesn't begin with a `/`.",
        );
        let diagnostic = CompilerDiagnostic::builder(err)
            .optional_source(source)
            .optional_label(label)
            .help(format!("Add a '/' at the beginning of the route path to fix this error: use `/{path}` instead of `{path}`."));
        diagnostics.push(diagnostic.build().into());
    }

    fn invalid_domain_guard(
        &self,
        location: &Location,
        e: InvalidDomainConstraint,
        package_graph: &PackageGraph,
        diagnostics: &mut Vec<miette::Error>,
    ) {
        let source = try_source!(location, package_graph, diagnostics);
        let label = source.as_ref().and_then(|source| {
            diagnostic::get_domain_span(source, location).labeled("The invalid domain".to_string())
        });
        let diagnostic = CompilerDiagnostic::builder(e)
            .optional_source(source)
            .optional_label(label);
        diagnostics.push(diagnostic.build().into());
    }

    fn path_prefix_cannot_be_empty(
        &self,
        location: &Location,
        package_graph: &PackageGraph,
        diagnostics: &mut Vec<miette::Error>,
    ) {
        let source = try_source!(location, package_graph, diagnostics);
        let label = source.as_ref().and_then(|source| {
            diagnostic::get_prefix_span(source, location).labeled("The empty prefix".to_string())
        });
        let err = anyhow!("Path prefixes cannot be empty.");
        let diagnostic = CompilerDiagnostic::builder(err)
            .optional_source(source)
            .optional_label(label)
            .help(
                "If you don't want to add a common prefix to all routes in the nested blueprint, \
                use the `nest` method directly."
                    .into(),
            );
        diagnostics.push(diagnostic.build().into());
    }

    fn path_prefix_must_start_with_a_slash(
        &self,
        prefix: &str,
        location: &Location,
        package_graph: &PackageGraph,
        diagnostics: &mut Vec<miette::Error>,
    ) {
        let source = try_source!(location, package_graph, diagnostics);
        let label = source.as_ref().and_then(|source| {
            diagnostic::get_prefix_span(source, location)
                .labeled("The prefix missing a leading '/'".to_string())
        });
        let err = anyhow!(
            "Path prefixes must begin with a forward slash, `/`.\n\
            `{prefix}` doesn't.",
        );
        let diagnostic = CompilerDiagnostic::builder(err)
            .optional_source(source)
            .optional_label(label)
            .help(format!("Add a '/' at the beginning of the path prefix to fix this error: use `/{prefix}` instead of `{prefix}`."));
        diagnostics.push(diagnostic.build().into());
    }

    fn path_prefix_cannot_end_with_a_slash(
        &self,
        prefix: &str,
        location: &Location,
        package_graph: &PackageGraph,
        diagnostics: &mut Vec<miette::Error>,
    ) {
        let source = try_source!(location, package_graph, diagnostics);
        let label = source.as_ref().and_then(|source| {
            diagnostic::get_prefix_span(source, location)
                .labeled("The prefix ending with a trailing '/'".to_string())
        });
        let err = anyhow!(
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
            .optional_label(label)
            .help(format!("Remove the '/' at the end of the path prefix to fix this error: use `{correct_prefix}` instead of `{prefix}`."));
        diagnostics.push(diagnostic.build().into());
    }
}
