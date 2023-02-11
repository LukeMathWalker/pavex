use pavex_builder::AppBlueprint;

use crate::web::analyses::raw_identifiers::{RawCallableIdentifierId, RawCallableIdentifiersDb};
use crate::web::interner::Interner;
use crate::web::resolvers::CallableType;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum UserComponent {
    RequestHandler {
        raw_callable_identifiers_id: RawCallableIdentifierId,
        route: String,
    },
    ErrorHandler {
        raw_callable_identifiers_id: RawCallableIdentifierId,
        fallible_callable_identifiers_id: UserComponentId,
    },
    Constructor {
        raw_callable_identifiers_id: RawCallableIdentifierId,
    },
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
    pub fn build(
        bp: &AppBlueprint,
        raw_callable_identifiers_db: &RawCallableIdentifiersDb,
    ) -> Self {
        let mut interner = Interner::new();
        for (route, request_handler) in &bp.router {
            let raw_callable_identifiers_id = raw_callable_identifiers_db[request_handler];
            let component = UserComponent::RequestHandler {
                raw_callable_identifiers_id,
                route: route.to_owned(),
            };
            let request_handler_id = interner.get_or_intern(component);
            if let Some(error_handler) = bp.request_handlers_error_handlers.get(route) {
                let raw_callable_identifiers_id = raw_callable_identifiers_db[error_handler];
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
