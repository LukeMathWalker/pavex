use ahash::{HashMap, HashMapExt};

use pavex_builder::{Blueprint, Lifecycle, Location, RawCallableIdentifiers};

use crate::compiler::analyses::scope_tree::{ScopeId, ScopeTree};
use crate::compiler::interner::Interner;

pub(crate) type RawCallableIdentifierId = la_arena::Idx<RawCallableIdentifiers>;

pub(crate) struct RawCallableIdentifiersDb {
    interner: Interner<RawCallableIdentifiers>,
    id2locations: HashMap<RawCallableIdentifierId, Location>,
    id2scope_id: HashMap<RawCallableIdentifierId, ScopeId<'static>>,
    id2lifecycle: HashMap<RawCallableIdentifierId, Lifecycle>,
}

impl RawCallableIdentifiersDb {
    pub fn build(bp: &Blueprint) -> (Self, ScopeTree) {
        let mut interner = Interner::new();
        let mut id2locations = HashMap::new();
        let mut id2lifecycle = HashMap::new();
        // We don't have the concept of nested scopes in Blueprint's API, but we are already
        // introducing it in the internal machinery.
        // In particular, all components belong to the root scope with the exception of
        // request handlers and their error handlers, which belong to a nested scope (one per
        // route).
        let mut id2scope_id = HashMap::new();
        let mut scope_tree = ScopeTree::new();
        let root_scope_id = scope_tree.root_scope_id().into_owned();

        for registered_route in &bp.routes {
            let route_scope_id = scope_tree.add_scope(root_scope_id.clone());
            let id = interner.get_or_intern(registered_route.request_handler.callable.clone());
            id2lifecycle.insert(id, Lifecycle::RequestScoped);
            id2locations.insert(id, registered_route.request_handler.location.to_owned());
            id2scope_id.insert(id, route_scope_id.clone());
            if let Some(error_handler) = &registered_route.error_handler {
                let error_handler_id = interner.get_or_intern(error_handler.callable.clone());
                id2lifecycle.insert(error_handler_id, Lifecycle::RequestScoped);
                id2locations.insert(error_handler_id, error_handler.location.to_owned());
                id2scope_id.insert(id, route_scope_id);
            }
        }

        for (fallible_constructor, error_handler) in &bp.constructors_error_handlers {
            let location = &bp.error_handler_locations[fallible_constructor];
            let error_handler_id = interner.get_or_intern(error_handler.to_owned());
            id2locations.insert(error_handler_id, location.to_owned());
            id2scope_id.insert(error_handler_id, root_scope_id.clone());
        }

        for constructor in &bp.constructors {
            let location = &bp.constructor_locations[constructor];
            let lifecycle = &bp.component_lifecycles[constructor];
            let id = interner.get_or_intern(constructor.to_owned());
            id2locations.insert(id, location.to_owned());
            id2lifecycle.insert(id, lifecycle.to_owned());
            id2scope_id.insert(id, root_scope_id.clone());
        }

        (
            Self {
                interner,
                id2locations,
                id2lifecycle,
                id2scope_id,
            },
            scope_tree,
        )
    }

    pub fn get_lifecycle(&self, id: RawCallableIdentifierId) -> Option<&Lifecycle> {
        self.id2lifecycle.get(&id)
    }

    pub fn get_location(&self, id: RawCallableIdentifierId) -> &Location {
        &self.id2locations[&id]
    }
}

impl std::ops::Index<RawCallableIdentifierId> for RawCallableIdentifiersDb {
    type Output = RawCallableIdentifiers;

    fn index(&self, index: RawCallableIdentifierId) -> &Self::Output {
        &self.interner[index]
    }
}

impl std::ops::Index<&RawCallableIdentifiers> for RawCallableIdentifiersDb {
    type Output = RawCallableIdentifierId;

    fn index(&self, index: &RawCallableIdentifiers) -> &Self::Output {
        &self.interner[index]
    }
}
