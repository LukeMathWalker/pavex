use super::{AnnotationId, ScopeId, blueprint::RawIdentifierId};

/// User components come from two sources:
///
/// - Direct registration against a `Blueprint` (e.g. via `bp.singleton`)
/// - Implicit registration of annotated components (e.g. via `#[pavex::constructor]`)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UserComponentSource {
    Blueprint(BlueprintSource),
    // We don't need to capture a `ScopeId` for annotated components
    // since they are always implicitly registered at the root scope.
    Annotation(AnnotationId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlueprintSource {
    pub identifiers_id: RawIdentifierId,
    pub scope_id: ScopeId,
}

impl From<BlueprintSource> for UserComponentSource {
    fn from(source: BlueprintSource) -> Self {
        UserComponentSource::Blueprint(source)
    }
}

impl From<AnnotationId> for UserComponentSource {
    fn from(id: AnnotationId) -> Self {
        UserComponentSource::Annotation(id)
    }
}
