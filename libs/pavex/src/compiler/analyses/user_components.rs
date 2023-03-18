use std::collections::BTreeSet;

use ahash::{HashMap, HashMapExt};

use pavex_builder::router::AllowedMethods;
use pavex_builder::{Blueprint, Lifecycle, Location, RawCallableIdentifiers};

use crate::compiler::analyses::scope_tree::{ScopeId, ScopeTree};
use crate::compiler::interner::Interner;
use crate::diagnostic::CallableType;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
/// A component registered by a framework user against the `Blueprint` for their application.
///
/// All components can be directly mapped back to the source code that registered them.
pub(crate) enum UserComponent {
    RequestHandler {
        raw_callable_identifiers_id: RawCallableIdentifierId,
        router_key: RouterKey,
        scope_id: ScopeId<'static>,
    },
    ErrorHandler {
        raw_callable_identifiers_id: RawCallableIdentifierId,
        fallible_callable_identifiers_id: UserComponentId,
        scope_id: ScopeId<'static>,
    },
    Constructor {
        raw_callable_identifiers_id: RawCallableIdentifierId,
        scope_id: ScopeId<'static>,
    },
}

/// A `RouterKey` uniquely identifies a subset of incoming requests for routing purposes.
/// Each request handler is associated with a `RouterKey`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RouterKey {
    pub path: String,
    /// If set to `Some(method_set)`, it will only match requests with an HTTP method that is
    /// present in the set.
    /// If set to `None`, it means that the handler matches all incoming requests for the given
    /// path, regardless of the HTTP method.
    pub method_guard: Option<BTreeSet<String>>,
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
        }
    }

    /// Returns an id that points at the raw identifiers for the callable that
    /// this `UserComponent` is associated with.
    pub fn raw_callable_identifiers_id(&self) -> RawCallableIdentifierId {
        match self {
            UserComponent::RequestHandler {
                raw_callable_identifiers_id,
                ..
            } => *raw_callable_identifiers_id,
            UserComponent::ErrorHandler {
                raw_callable_identifiers_id,
                ..
            } => *raw_callable_identifiers_id,
            UserComponent::Constructor {
                raw_callable_identifiers_id,
                ..
            } => *raw_callable_identifiers_id,
        }
    }

    /// Returns the raw identifiers for the callable that this `UserComponent` is associated with.
    pub fn raw_callable_identifiers<'b>(
        &self,
        db: &'b UserComponentDb,
    ) -> &'b RawCallableIdentifiers {
        &db.identifiers_interner[self.raw_callable_identifiers_id()]
    }
}

/// A unique identifier for a `RawCallableIdentifiers`.
pub(crate) type RawCallableIdentifierId = la_arena::Idx<RawCallableIdentifiers>;

/// A unique identifier for a [`UserComponent`].
pub(crate) type UserComponentId = la_arena::Idx<UserComponent>;

/// A database that contains all the user components that have been registered against the
/// `Blueprint` for the application.
///
/// For each component, we keep track of:
/// - the raw identifiers for the callable that it is associated with;
/// - the source code location where it was registered (for error reporting purposes);
/// - the lifecycle of the component;
/// - the scope that the component belongs to.
pub(crate) struct UserComponentDb {
    component_interner: Interner<UserComponent>,
    identifiers_interner: Interner<RawCallableIdentifiers>,
    id2locations: HashMap<UserComponentId, Location>,
    id2lifecycle: HashMap<UserComponentId, Lifecycle>,
    scope_tree: ScopeTree,
}

impl UserComponentDb {
    /// Process a `Blueprint` and return a `UserComponentDb` that contains all the user components
    /// that have been registered against it.
    pub fn build(bp: &Blueprint) -> Self {
        let mut identifiers_interner = Interner::new();
        let mut component_interner = Interner::new();
        let mut id2locations = HashMap::new();
        let mut id2lifecycle = HashMap::new();
        // We don't have the concept of nested scopes in Blueprint's API, but we are already
        // introducing it in the internal machinery.
        // In particular, all components belong to the root scope with the exception of
        // request handlers and their error handlers, which belong to a nested scope (one per
        // route).
        let mut scope_tree = ScopeTree::new();
        let root_scope_id = scope_tree.root_scope_id().into_owned();

        for registered_route in &bp.routes {
            let raw_callable_identifiers_id = identifiers_interner
                .get_or_intern(registered_route.request_handler.callable.clone());
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
            let route_scope_id = scope_tree.add_scope(root_scope_id.clone());
            let component = UserComponent::RequestHandler {
                raw_callable_identifiers_id,
                router_key: RouterKey {
                    path: registered_route.path.to_owned(),
                    method_guard,
                },
                scope_id: route_scope_id.clone(),
            };
            let request_handler_id = component_interner.get_or_intern(component);
            id2lifecycle.insert(request_handler_id, Lifecycle::RequestScoped);
            id2locations.insert(
                request_handler_id,
                registered_route.request_handler.location.to_owned(),
            );

            if let Some(error_handler) = &registered_route.error_handler {
                let raw_callable_identifiers_id =
                    identifiers_interner.get_or_intern(error_handler.callable.clone());
                let component = UserComponent::ErrorHandler {
                    raw_callable_identifiers_id,
                    fallible_callable_identifiers_id: request_handler_id,
                    scope_id: route_scope_id,
                };
                let error_handler_id = component_interner.get_or_intern(component);
                id2lifecycle.insert(error_handler_id, Lifecycle::RequestScoped);
                id2locations.insert(error_handler_id, error_handler.location.to_owned());
            }
        }

        for constructor in &bp.constructors {
            let raw_callable_identifiers_id =
                identifiers_interner.get_or_intern(constructor.clone());
            let component = UserComponent::Constructor {
                raw_callable_identifiers_id,
                scope_id: root_scope_id.clone(),
            };
            let constructor_id = component_interner.get_or_intern(component);
            let location = &bp.constructor_locations[constructor];
            id2locations.insert(constructor_id, location.to_owned());
            let lifecycle = &bp.component_lifecycles[constructor];
            id2lifecycle.insert(constructor_id, lifecycle.to_owned());
            if let Some(error_handler) = bp.constructors_error_handlers.get(constructor) {
                let raw_callable_identifiers_id =
                    identifiers_interner.get_or_intern(error_handler.clone());
                let component = UserComponent::ErrorHandler {
                    raw_callable_identifiers_id,
                    fallible_callable_identifiers_id: constructor_id,
                    scope_id: root_scope_id.clone(),
                };
                let error_handler_id = component_interner.get_or_intern(component);
                id2lifecycle.insert(error_handler_id, lifecycle.to_owned());
                let location = &bp.error_handler_locations[constructor];
                id2locations.insert(error_handler_id, location.to_owned());
            }
        }

        Self {
            component_interner,
            identifiers_interner,
            id2locations,
            id2lifecycle,
            scope_tree,
        }
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
    ) -> impl Iterator<Item = (UserComponentId, &UserComponent)> + ExactSizeIterator + DoubleEndedIterator
    {
        self.component_interner
            .iter()
            .filter(|(_, c)| matches!(c, UserComponent::Constructor { .. }))
    }

    /// Iterate over all the request handler components in the database, returning their id and the
    /// associated `UserComponent`.
    pub fn request_handlers(
        &self,
    ) -> impl Iterator<Item = (UserComponentId, &UserComponent)> + ExactSizeIterator + DoubleEndedIterator
    {
        self.component_interner
            .iter()
            .filter(|(_, c)| matches!(c, UserComponent::RequestHandler { .. }))
    }

    /// Return the lifecycle of the component with the given id.
    pub fn get_lifecycle(&self, id: UserComponentId) -> &Lifecycle {
        &self.id2lifecycle[&id]
    }

    /// Return the location where the component with the given id was registered against the
    /// application blueprint.
    pub fn get_location(&self, id: UserComponentId) -> &Location {
        &self.id2locations[&id]
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
