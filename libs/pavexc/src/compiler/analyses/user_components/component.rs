use crate::diagnostic::ComponentKind;

use super::{
    ScopeId, UserComponentSource, blueprint::RawIdentifierId, router_key::RouterKey,
    source::BlueprintSource,
};

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
        source: BlueprintSource,
        router_key: RouterKey,
    },
    Fallback {
        source: BlueprintSource,
    },
    ErrorHandler {
        source: BlueprintSource,
        fallible_id: UserComponentId,
    },
    Constructor {
        source: UserComponentSource,
    },
    PrebuiltType {
        source: BlueprintSource,
    },
    ConfigType {
        source: BlueprintSource,
        key: String,
    },
    WrappingMiddleware {
        source: BlueprintSource,
    },
    PostProcessingMiddleware {
        source: BlueprintSource,
    },
    PreProcessingMiddleware {
        source: BlueprintSource,
    },
    ErrorObserver {
        source: BlueprintSource,
    },
}

impl UserComponent {
    /// Returns the raw identifiers id for this user component.
    ///
    /// It's `None` for annotated components.
    pub fn raw_identifiers_id(&self) -> Option<RawIdentifierId> {
        self.bp_source().map(|s| s.identifiers_id)
    }

    /// Returns the blueprint source for this user component.
    ///
    /// It's `None` for annotated components.
    pub fn bp_source(&self) -> Option<BlueprintSource> {
        match self {
            UserComponent::RequestHandler { source, .. }
            | UserComponent::Fallback { source }
            | UserComponent::ErrorHandler { source, .. }
            | UserComponent::PrebuiltType { source }
            | UserComponent::ConfigType { source, .. }
            | UserComponent::WrappingMiddleware { source }
            | UserComponent::PostProcessingMiddleware { source }
            | UserComponent::PreProcessingMiddleware { source }
            | UserComponent::Constructor {
                source: UserComponentSource::Blueprint(source),
            }
            | UserComponent::ErrorObserver { source } => Some(*source),
            _ => None,
        }
    }

    /// Returns the [`ScopeId`] for the scope that this [`UserComponent`] is associated with.
    pub fn scope_id(&self) -> ScopeId {
        match self.bp_source() {
            Some(source) => source.scope_id,
            None => ScopeId::ROOT,
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
            Fallback { .. } => ComponentKind::RequestHandler,
            ErrorObserver { .. } => ComponentKind::ErrorObserver,
            PostProcessingMiddleware { .. } => ComponentKind::PostProcessingMiddleware,
            PreProcessingMiddleware { .. } => ComponentKind::PreProcessingMiddleware,
        }
    }
}
