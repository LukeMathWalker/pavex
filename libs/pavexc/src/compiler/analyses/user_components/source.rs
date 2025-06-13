use super::{AnnotatedItemId, annotations::AnnotationCoordinatesId, blueprint::RawIdentifierId};

/// Information about the source of a given user component.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UserComponentSource {
    /// The component was registered using a `Blueprint` method
    /// (e.g. via `bp.singleton`) and the "old-school" path-based
    /// identification system.
    Identifiers(RawIdentifierId),
    /// The component was registered using a `Blueprint` method
    /// (e.g. via `bp.wrap`).
    AnnotationCoordinates(AnnotationCoordinatesId),
    /// The component was imported.
    Annotation(AnnotatedItemId),
}

impl From<RawIdentifierId> for UserComponentSource {
    fn from(v: RawIdentifierId) -> Self {
        Self::Identifiers(v)
    }
}

impl From<AnnotationCoordinatesId> for UserComponentSource {
    fn from(c: AnnotationCoordinatesId) -> Self {
        Self::AnnotationCoordinates(c)
    }
}

impl From<AnnotatedItemId> for UserComponentSource {
    fn from(s: AnnotatedItemId) -> Self {
        Self::Annotation(s)
    }
}
