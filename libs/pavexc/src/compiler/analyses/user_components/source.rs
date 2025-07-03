use super::{AnnotatedItemId, annotations::AnnotationCoordinatesId};

/// Information about the source of a given user component.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UserComponentSource {
    /// The component was registered using a `Blueprint` method
    /// (e.g. via `bp.wrap`).
    BlueprintRegistration(AnnotationCoordinatesId),
    /// The component was imported, either via `bp.import` or `bp.routes`.
    Import(AnnotatedItemId),
}

impl From<AnnotationCoordinatesId> for UserComponentSource {
    fn from(c: AnnotationCoordinatesId) -> Self {
        Self::BlueprintRegistration(c)
    }
}

impl From<AnnotatedItemId> for UserComponentSource {
    fn from(s: AnnotatedItemId) -> Self {
        Self::Import(s)
    }
}
