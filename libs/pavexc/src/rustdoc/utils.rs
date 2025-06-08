// Ensure that crate names are in canonical form! Damn automated hyphen substitution!
pub fn normalize_crate_name(s: &str) -> String {
    s.replace('-', "_")
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(from = "u32", into = "u32")]
/// A wrapper around [`rustdoc_types::Id`] to make it sortable.
///
/// TODO: Remove when/if https://github.com/rust-lang/rust/pull/141898 is merged.
pub struct SortableId(pub rustdoc_types::Id);

impl From<u32> for SortableId {
    fn from(value: u32) -> Self {
        Self(rustdoc_types::Id(value))
    }
}

impl From<SortableId> for u32 {
    fn from(value: SortableId) -> Self {
        value.0.0
    }
}

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
