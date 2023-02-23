use std::collections::BTreeSet;

use pavex_builder::router::AllowedMethods;
use pavex_builder::Blueprint;

use crate::diagnostic::CallableType;
use crate::web::analyses::raw_identifiers::{RawCallableIdentifierId, RawCallableIdentifiersDb};
use crate::web::interner::Interner;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum UserComponent {
    RequestHandler {
        raw_callable_identifiers_id: RawCallableIdentifierId,
        router_key: RouterKey,
    },
    ErrorHandler {
        raw_callable_identifiers_id: RawCallableIdentifierId,
        fallible_callable_identifiers_id: UserComponentId,
    },
    Constructor {
        raw_callable_identifiers_id: RawCallableIdentifierId,
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
    pub fn callable_type(&self) -> CallableType {
        match self {
            UserComponent::RequestHandler { .. } => CallableType::RequestHandler,
            UserComponent::ErrorHandler { .. } => CallableType::ErrorHandler,
            UserComponent::Constructor { .. } => CallableType::Constructor,
        }
    }
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
            } => *raw_callable_identifiers_id,
        }
    }
}

pub(crate) type UserComponentId = la_arena::Idx<UserComponent>;

pub(crate) struct UserComponentDb {
    interner: Interner<UserComponent>,
}

impl UserComponentDb {
    pub fn build(bp: &Blueprint, raw_callable_identifiers_db: &RawCallableIdentifiersDb) -> Self {
        let mut interner = Interner::new();
        for registered_route in &bp.routes {
            let raw_callable_identifiers_id =
                raw_callable_identifiers_db[&registered_route.request_handler.callable];
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
            let component = UserComponent::RequestHandler {
                raw_callable_identifiers_id,
                router_key: RouterKey {
                    path: registered_route.path.to_owned(),
                    method_guard,
                },
            };
            let request_handler_id = interner.get_or_intern(component);
            if let Some(error_handler) = &registered_route.error_handler {
                let raw_callable_identifiers_id =
                    raw_callable_identifiers_db[&error_handler.callable];
                let component = UserComponent::ErrorHandler {
                    raw_callable_identifiers_id,
                    fallible_callable_identifiers_id: request_handler_id,
                };
                interner.get_or_intern(component);
            }
        }

        for constructor in &bp.constructors {
            let raw_callable_identifiers_id = raw_callable_identifiers_db[constructor];
            let component = UserComponent::Constructor {
                raw_callable_identifiers_id,
            };
            let constructor_id = interner.get_or_intern(component);
            if let Some(error_handler) = bp.constructors_error_handlers.get(constructor) {
                let raw_callable_identifiers_id = raw_callable_identifiers_db[error_handler];
                let component = UserComponent::ErrorHandler {
                    raw_callable_identifiers_id,
                    fallible_callable_identifiers_id: constructor_id,
                };
                interner.get_or_intern(component);
            }
        }
        Self { interner }
    }

    pub fn iter(
        &self,
    ) -> impl Iterator<Item = (UserComponentId, &UserComponent)> + ExactSizeIterator + DoubleEndedIterator
    {
        self.interner.iter()
    }
}

impl std::ops::Index<UserComponentId> for UserComponentDb {
    type Output = UserComponent;

    fn index(&self, index: UserComponentId) -> &Self::Output {
        &self.interner[index]
    }
}

impl std::ops::Index<&UserComponent> for UserComponentDb {
    type Output = UserComponentId;

    fn index(&self, index: &UserComponent) -> &Self::Output {
        &self.interner[index]
    }
}
