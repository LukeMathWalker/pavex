use crate::diagnostic::ComponentKind;

use super::{UserComponentSource, blueprint::RawIdentifierId, router_key::RouterKey};

/// A unique identifier for a [`UserComponent`].
pub type UserComponentId = la_arena::Idx<UserComponent>;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
/// A component registered by a user either via a macro annotation or directly against
/// their `Blueprint`.
///
/// All user components can be directly mapped back to the source code that registered them.
///
/// See [`UserComponentDb`] for more details.
///
/// [`UserComponentDb`]: super::UserComponentDb
pub enum UserComponent {
    RequestHandler {
        source: RawIdentifierId,
        router_key: RouterKey,
    },
    Fallback {
        source: RawIdentifierId,
    },
    ErrorHandler {
        source: UserComponentSource,
        fallible_id: UserComponentId,
    },
    Constructor {
        source: UserComponentSource,
    },
    PrebuiltType {
        source: RawIdentifierId,
    },
    ConfigType {
        source: UserComponentSource,
        key: String,
    },
    WrappingMiddleware {
        source: RawIdentifierId,
    },
    PostProcessingMiddleware {
        source: RawIdentifierId,
    },
    PreProcessingMiddleware {
        source: RawIdentifierId,
    },
    ErrorObserver {
        source: RawIdentifierId,
    },
}

impl UserComponent {
    /// Returns the raw identifiers id for this user component.
    ///
    /// It's `None` for annotated components.
    pub fn raw_identifiers_id(&self) -> Option<RawIdentifierId> {
        match self {
            UserComponent::RequestHandler { source, .. }
            | UserComponent::Fallback { source }
            | UserComponent::PrebuiltType { source }
            | UserComponent::WrappingMiddleware { source }
            | UserComponent::PostProcessingMiddleware { source }
            | UserComponent::PreProcessingMiddleware { source }
            | UserComponent::ConfigType {
                source: UserComponentSource::Identifiers(source),
                ..
            }
            | UserComponent::Constructor {
                source: UserComponentSource::Identifiers(source),
            }
            | UserComponent::ErrorHandler {
                source: UserComponentSource::Identifiers(source),
                ..
            }
            | UserComponent::ErrorObserver { source } => Some(*source),
            _ => None,
        }
    }

    /// Returns the tag for the "variant" of this [`UserComponent`].
    ///
    /// Useful when you don't need to access the actual data attached component.
    pub fn kind(&self) -> ComponentKind {
        use UserComponent::*;

        match self {
            RequestHandler { .. } => ComponentKind::RequestHandler,
            ErrorHandler { .. } => ComponentKind::ErrorHandler,
            Constructor { .. } => ComponentKind::Constructor,
            PrebuiltType { .. } => ComponentKind::PrebuiltType,
            ConfigType { .. } => ComponentKind::ConfigType,
            WrappingMiddleware { .. } => ComponentKind::WrappingMiddleware,
            Fallback { .. } => ComponentKind::Fallback,
            ErrorObserver { .. } => ComponentKind::ErrorObserver,
            PostProcessingMiddleware { .. } => ComponentKind::PostProcessingMiddleware,
            PreProcessingMiddleware { .. } => ComponentKind::PreProcessingMiddleware,
        }
    }
}
