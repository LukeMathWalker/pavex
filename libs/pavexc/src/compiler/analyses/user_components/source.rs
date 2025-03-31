use super::{AnnotatedItemId, blueprint::RawIdentifierId};

/// User components come from two sources:
///
/// - Direct registration against a `Blueprint` (e.g. via `bp.singleton`)
/// - Imports of annotated components (e.g. via `#[pavex::constructor]`)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UserComponentSource {
    Identifiers(RawIdentifierId),
    Annotation(AnnotatedItemId),
}

impl From<RawIdentifierId> for UserComponentSource {
    fn from(v: RawIdentifierId) -> Self {
        UserComponentSource::Identifiers(v)
    }
}

impl From<AnnotatedItemId> for UserComponentSource {
    fn from(s: AnnotatedItemId) -> Self {
        UserComponentSource::Annotation(s)
    }
}
