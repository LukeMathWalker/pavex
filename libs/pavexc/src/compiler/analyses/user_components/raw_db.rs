use std::collections::BTreeSet;

use ahash::{HashMap, HashMapExt};
use anyhow::anyhow;
use guppy::graph::PackageGraph;

use pavex::blueprint::constructor::CloningStrategy;
use pavex::blueprint::internals::{
    NestedBlueprint, RegisteredCallable, RegisteredConstructor, RegisteredRoute,
    RegisteredWrappingMiddleware,
};
use pavex::blueprint::router::AllowedMethods;
use pavex::blueprint::{
    constructor::Lifecycle, reflection::Location, reflection::RawCallableIdentifiers, Blueprint,
};

use crate::compiler::analyses::user_components::router_key::RouterKey;
use crate::compiler::analyses::user_components::scope_graph::ScopeGraphBuilder;
use crate::compiler::analyses::user_components::{ScopeGraph, ScopeId};
use crate::compiler::interner::Interner;
use crate::diagnostic;
use crate::diagnostic::{CallableType, CompilerDiagnostic, LocationExt, SourceSpanExt};

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
        raw_callable_identifiers_id: RawCallableIdentifierId,
        router_key: RouterKey,
        scope_id: ScopeId,
    },
    ErrorHandler {
        raw_callable_identifiers_id: RawCallableIdentifierId,
        fallible_callable_identifiers_id: UserComponentId,
        scope_id: ScopeId,
    },
    Constructor {
        raw_callable_identifiers_id: RawCallableIdentifierId,
        scope_id: ScopeId,
    },
    WrappingMiddleware {
        raw_callable_identifiers_id: RawCallableIdentifierId,
        scope_id: ScopeId,
    },
}

impl UserComponent {
    /// Returns the tag for the "variant" of this `UserComponent`.
    ///
    /// Useful when you don't need to access the actual data attached component.
    pub fn callable_type(&self) -> CallableType {
        match self {
            UserComponent::RequestHandler { .. } => CallableType::RequestHandler,
            UserComponent::ErrorHandler { .. } => CallableType::ErrorHandler,
            UserComponent::Constructor { .. } => CallableType::Constructor,
            UserComponent::WrappingMiddleware { .. } => CallableType::WrappingMiddleware,
        }
    }

    /// Returns an id that points at the raw identifiers for the callable that
    /// this [`UserComponent`] is associated with.
    pub fn raw_callable_identifiers_id(&self) -> RawCallableIdentifierId {
        match self {
            UserComponent::WrappingMiddleware {
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
            | UserComponent::Constructor {
                raw_callable_identifiers_id,
                ..
            } => *raw_callable_identifiers_id,
        }
    }

    /// Returns the [`ScopeId`] for the scope that this [`UserComponent`] is associated with.
    pub fn scope_id(&self) -> ScopeId {
        match self {
            UserComponent::RequestHandler { scope_id, .. }
            | UserComponent::ErrorHandler { scope_id, .. }
            | UserComponent::WrappingMiddleware { scope_id, .. }
            | UserComponent::Constructor { scope_id, .. } => *scope_id,
        }
    }

    /// Returns the raw identifiers for the callable that this `UserComponent` is associated with.
    pub(super) fn raw_callable_identifiers<'b>(
        &self,
        db: &'b RawUserComponentDb,
    ) -> &'b RawCallableIdentifiers {
        &db.identifiers_interner[self.raw_callable_identifiers_id()]
    }
}

/// A unique identifier for a `RawCallableIdentifiers`.
pub type RawCallableIdentifierId = la_arena::Idx<RawCallableIdentifiers>;

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
    pub(super) identifiers_interner: Interner<RawCallableIdentifiers>,
    /// Associate each user-registered component with the location it was
    /// registered at against the `Blueprint` in the user's source code.
    ///
    /// Invariants: there is an entry for every single user component.
    pub(super) id2locations: HashMap<UserComponentId, Location>,
    /// Associate each user-registered component with its lifecycle.
    ///
    /// Invariants: there is an entry for every single user component.
    pub(super) id2lifecycle: HashMap<UserComponentId, Lifecycle>,
    /// For each constructor component, determine if it can be cloned or not.
    ///
    /// Invariants: there is an entry for every constructor.
    pub(super) constructor_id2cloning_strategy: HashMap<UserComponentId, CloningStrategy>,
    /// Associate each request handler with the ordered list of middlewares that wrap around it.
    ///
    /// Invariants: there is an entry for every single request handler.
    pub(super) handler_id2middleware_ids: HashMap<UserComponentId, Vec<UserComponentId>>,
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
            constructor_id2cloning_strategy: HashMap::new(),
            handler_id2middleware_ids: HashMap::new(),
        };
        let mut scope_graph_builder = ScopeGraph::builder(bp.creation_location.clone());
        let root_scope_id = scope_graph_builder.root_scope_id();
        // The middleware chain that will wrap around all the request handlers in the current scope.
        // We discover and add more middlewares to this chain as we process the root blueprint and
        // its nested blueprints.
        // By default, the middleware chain is empty.
        let mut current_middleware_chain = Vec::new();

        Self::process_blueprint(
            &mut self_,
            bp,
            root_scope_id,
            None,
            &mut scope_graph_builder,
            &mut current_middleware_chain,
            package_graph,
            diagnostics,
        );

        struct QueueItem<'a> {
            parent_scope_id: ScopeId,
            parent_path_prefix: Option<String>,
            nested_bp: &'a NestedBlueprint,
            current_middleware_chain: Vec<UserComponentId>,
        }
        let mut processing_queue: Vec<_> = bp
            .nested_blueprints
            .iter()
            .map(|nested_bp| QueueItem {
                parent_scope_id: root_scope_id,
                nested_bp: &nested_bp,
                parent_path_prefix: None,
                current_middleware_chain: current_middleware_chain.clone(),
            })
            .collect();

        while let Some(item) = processing_queue.pop() {
            let QueueItem {
                parent_scope_id,
                nested_bp,
                parent_path_prefix,
                mut current_middleware_chain,
            } = item;
            let nested_scope_id = scope_graph_builder
                .add_scope(parent_scope_id, Some(nested_bp.nesting_location.clone()));
            self_.validate_nested_bp(nested_bp, package_graph, diagnostics);

            let path_prefix = match parent_path_prefix {
                Some(prefix) => Some(format!(
                    "{}{}",
                    prefix,
                    nested_bp.path_prefix.as_deref().unwrap_or("")
                )),
                None => nested_bp.path_prefix.clone(),
            };

            Self::process_blueprint(
                &mut self_,
                &nested_bp.blueprint,
                nested_scope_id,
                path_prefix.as_deref(),
                &mut scope_graph_builder,
                &mut current_middleware_chain,
                package_graph,
                diagnostics,
            );
            for nested_bp in &nested_bp.blueprint.nested_blueprints {
                processing_queue.push(QueueItem {
                    parent_scope_id: nested_scope_id,
                    nested_bp,
                    parent_path_prefix: path_prefix.clone(),
                    current_middleware_chain: current_middleware_chain.clone(),
                });
            }
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
    fn process_blueprint(
        &mut self,
        bp: &Blueprint,
        current_scope_id: ScopeId,
        path_prefix: Option<&str>,
        scope_graph_builder: &mut ScopeGraphBuilder,
        current_middleware_chain: &mut Vec<UserComponentId>,
        package_graph: &PackageGraph,
        diagnostics: &mut Vec<miette::Error>,
    ) {
        self.process_middlewares(&bp.middlewares, current_scope_id, current_middleware_chain);
        self.process_routes(
            &bp.routes,
            &current_middleware_chain,
            current_scope_id,
            path_prefix,
            scope_graph_builder,
            package_graph,
            diagnostics,
        );
        self.process_constructors(&bp.constructors, current_scope_id);
    }

    /// Register with [`RawUserComponentDb`] all the routes that have been
    /// registered against the provided `Blueprint`, including their error handlers
    /// (if present).  
    fn process_routes(
        &mut self,
        routes: &[RegisteredRoute],
        current_middleware_chain: &[UserComponentId],
        current_scope_id: ScopeId,
        path_prefix: Option<&str>,
        scope_graph_builder: &mut ScopeGraphBuilder,
        package_graph: &PackageGraph,
        diagnostics: &mut Vec<miette::Error>,
    ) {
        const ROUTE_LIFECYCLE: Lifecycle = Lifecycle::RequestScoped;

        for registered_route in routes {
            let raw_callable_identifiers_id = self
                .identifiers_interner
                .get_or_intern(registered_route.request_handler.callable.clone());
            let route_scope_id = scope_graph_builder.add_scope(current_scope_id, None);
            let router_key = {
                let method_guard = match &registered_route.method_guard.allowed_methods {
                    AllowedMethods::All => None,
                    AllowedMethods::Single(m) => {
                        let mut set = BTreeSet::new();
                        set.insert(m.to_string());
                        Some(set)
                    }
                    AllowedMethods::Multiple(methods) => {
                        methods.iter().map(|m| Some(m.to_string())).collect()
                    }
                };
                let path = match path_prefix {
                    Some(prefix) => format!("{}{}", prefix, registered_route.path),
                    None => registered_route.path.to_owned(),
                };
                RouterKey { path, method_guard }
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
    }

    /// Register with [`RawUserComponentDb`] all the routes that have been
    /// registered against the provided `Blueprint`, including their error handlers
    /// (if present).  
    fn process_middlewares(
        &mut self,
        middlewares: &[RegisteredWrappingMiddleware],
        current_scope_id: ScopeId,
        current_middleware_chain: &mut Vec<UserComponentId>,
    ) {
        const MIDDLEWARE_LIFECYCLE: Lifecycle = Lifecycle::RequestScoped;

        for middleware in middlewares {
            let raw_callable_identifiers_id = self
                .identifiers_interner
                .get_or_intern(middleware.middleware.callable.clone());
            let component = UserComponent::WrappingMiddleware {
                raw_callable_identifiers_id,
                scope_id: current_scope_id,
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
    }

    /// Register with [`RawUserComponentDb`] all the constructors that have been
    /// registered against the provided `Blueprint`, including their error handlers
    /// (if present).  
    /// All components are associated with or nested under the provided `current_scope_id`.
    fn process_constructors(
        &mut self,
        constructors: &[RegisteredConstructor],
        current_scope_id: ScopeId,
    ) {
        for constructor in constructors {
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
            self.constructor_id2cloning_strategy.insert(
                constructor_id,
                constructor
                    .cloning_strategy
                    .unwrap_or(CloningStrategy::NeverClone),
            );

            self.process_error_handler(
                &constructor.error_handler,
                lifecycle,
                current_scope_id,
                constructor_id,
            );
        }
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
        error_handler: &Option<RegisteredCallable>,
        lifecycle: Lifecycle,
        scope_id: ScopeId,
        fallible_component_id: UserComponentId,
    ) {
        let Some(error_handler) = error_handler else { return; };
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
        route: &RegisteredRoute,
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

    /// Check the path prefix of the nested blueprint.
    /// Emit diagnostics if the path prefix is invalid—i.e. empty or missing a leading slash.
    fn validate_nested_bp(
        &self,
        nested_bp: &NestedBlueprint,
        package_graph: &PackageGraph,
        diagnostics: &mut Vec<miette::Error>,
    ) {
        if let Some(path_prefix) = nested_bp.path_prefix.as_deref() {
            if path_prefix.is_empty() {
                self.path_prefix_cannot_be_empty(nested_bp, package_graph, diagnostics);
                return;
            }

            if !path_prefix.starts_with('/') {
                self.path_prefix_must_start_with_a_slash(nested_bp, package_graph, diagnostics);
            }

            if path_prefix.ends_with('/') {
                self.path_prefix_cannot_end_with_a_slash(nested_bp, package_graph, diagnostics);
            }
        }
    }

    /// Validate that all internal invariants are satisfied.
    #[cfg(debug_assertions)]
    fn check_invariants(&self) {
        for (id, component) in self.iter() {
            assert!(
                self.id2lifecycle.get(&id).is_some(),
                "There is no lifecycle registered for the user component #{:?}",
                id
            );
            assert!(
                self.id2locations.get(&id).is_some(),
                "There is no location registered for the user component #{:?}",
                id
            );
            match component {
                UserComponent::Constructor { .. } => {
                    assert!(
                        self.constructor_id2cloning_strategy.get(&id).is_some(),
                        "There is no cloning strategy registered for the user-registered constructor #{:?}",
                        id
                    );
                }
                UserComponent::RequestHandler { .. } => {
                    assert!(
                        self.handler_id2middleware_ids.get(&id).is_some(),
                        "The middleware chain is missing for the user-registered request handler #{:?}",
                        id
                    );
                }
                UserComponent::ErrorHandler { .. } | UserComponent::WrappingMiddleware { .. } => {}
            }
        }
    }
}

impl RawUserComponentDb {
    /// Iterate over all the user components in the database, returning their id and the associated
    /// `UserComponent`.
    pub fn iter(
        &self,
    ) -> impl Iterator<Item = (UserComponentId, &UserComponent)> + ExactSizeIterator + DoubleEndedIterator
    {
        self.component_interner.iter()
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
        route: &RegisteredRoute,
        route_id: UserComponentId,
        package_graph: &PackageGraph,
        diagnostics: &mut Vec<miette::Error>,
    ) {
        let location = self.get_location(route_id);
        let source = match location.source_file(package_graph) {
            Ok(source) => source,
            Err(e) => {
                diagnostics.push(e.into());
                return;
            }
        };
        let label = diagnostic::get_route_path_span(&source, location)
            .map(|s| s.labeled("The path missing a leading '/'".to_string()));
        let path = &route.path;
        let err =
            anyhow!("All route paths must begin with a forward slash, `/`.\n`{path}` doesn't.",);
        let diagnostic = CompilerDiagnostic::builder(source, err)
            .optional_label(label)
            .help(format!("Add a '/' at the beginning of the route path to fix this error: use `/{path}` instead of `{path}`."));
        diagnostics.push(diagnostic.build().into());
    }

    fn path_prefix_cannot_be_empty(
        &self,
        nested_bp: &NestedBlueprint,
        package_graph: &PackageGraph,
        diagnostics: &mut Vec<miette::Error>,
    ) {
        let location = &nested_bp.nesting_location;
        let source = match location.source_file(package_graph) {
            Ok(source) => source,
            Err(e) => {
                diagnostics.push(e.into());
                return;
            }
        };
        let label = diagnostic::get_nest_at_prefix_span(&source, location)
            .map(|s| s.labeled("The empty prefix".to_string()));
        let err = anyhow!("The path prefix passed to `nest_at` cannot be empty.");
        let diagnostic = CompilerDiagnostic::builder(source, err)
            .optional_label(label)
            .help(
                "If you don't want to add a common prefix to all routes in the nested blueprint, \
                use the `nest` method instead of `nest_at`."
                    .into(),
            );
        diagnostics.push(diagnostic.build().into());
    }

    fn path_prefix_must_start_with_a_slash(
        &self,
        nested_bp: &NestedBlueprint,
        package_graph: &PackageGraph,
        diagnostics: &mut Vec<miette::Error>,
    ) {
        let location = &nested_bp.nesting_location;
        let source = match location.source_file(package_graph) {
            Ok(source) => source,
            Err(e) => {
                diagnostics.push(e.into());
                return;
            }
        };
        let label = diagnostic::get_nest_at_prefix_span(&source, location)
            .map(|s| s.labeled("The prefix missing a leading '/'".to_string()));
        let prefix = nested_bp.path_prefix.as_deref().unwrap();
        let err = anyhow!(
            "The path prefix passed to `nest_at` must begin with a forward slash, `/`.\n\
            `{prefix}` doesn't.",
        );
        let diagnostic = CompilerDiagnostic::builder(source, err)
            .optional_label(label)
            .help(format!("Add a '/' at the beginning of the path prefix to fix this error: use `/{prefix}` instead of `{prefix}`."));
        diagnostics.push(diagnostic.build().into());
    }

    fn path_prefix_cannot_end_with_a_slash(
        &self,
        nested_bp: &NestedBlueprint,
        package_graph: &PackageGraph,
        diagnostics: &mut Vec<miette::Error>,
    ) {
        let location = &nested_bp.nesting_location;
        let source = match location.source_file(package_graph) {
            Ok(source) => source,
            Err(e) => {
                diagnostics.push(e.into());
                return;
            }
        };
        let label = diagnostic::get_nest_at_prefix_span(&source, location)
            .map(|s| s.labeled("The prefix ending with a trailing '/'".to_string()));
        let prefix = nested_bp.path_prefix.as_deref().unwrap();
        let err = anyhow!(
            "The path prefix passed to `nest_at` can't end with a trailing slash, `/`. \
            `{prefix}` does.\n\
            Trailing slashes in path prefixes increase the likelihood of having consecutive \
            slashes in the final route path, which is rarely desireable. If you want consecutive \
            slashes in the final route path, you can add them explicitly in the paths of the routes \
            registered against the nested blueprint.",
        );
        let correct_prefix = prefix.trim_end_matches('/');
        let diagnostic = CompilerDiagnostic::builder(source, err)
            .optional_label(label)
            .help(format!("Remove the '/' at the end of the path prefix to fix this error: use `{correct_prefix}` instead of `{prefix}`."));
        diagnostics.push(diagnostic.build().into());
    }
}
