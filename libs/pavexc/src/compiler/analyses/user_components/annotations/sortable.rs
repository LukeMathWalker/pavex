#[derive(Debug, Copy, Clone, PartialEq, Eq)]
/// A wrapper around [`rustdoc_types::Id`] to make it sortable.
pub struct SortableId(pub rustdoc_types::Id);

impl From<rustdoc_types::Id> for SortableId {
    fn from(value: rustdoc_types::Id) -> Self {
        Self(value)
    }
}

impl PartialOrd for SortableId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SortableId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.0.cmp(&other.0.0)
    }
}
