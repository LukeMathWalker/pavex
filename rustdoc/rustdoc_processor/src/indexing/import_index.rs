//! Index of importable items in a crate.

use std::cmp::Ordering;
use std::collections::BTreeSet;

use ahash::HashMap;

/// An index of all importable items in a crate.
#[derive(
    Debug, Clone, Default, serde::Serialize, serde::Deserialize, bincode::Encode, bincode::Decode,
)]
pub struct ImportIndex {
    /// A mapping that keeps track of all modules defined in the current crate.
    ///
    /// We track modules separately because their names are allowed to collide with
    /// type and function names.
    pub modules: HashMap<rustdoc_types::Id, ImportIndexEntry>,
    /// A mapping that keeps track of traits, structs, enums and functions
    /// defined in the current crate.
    pub items: HashMap<rustdoc_types::Id, ImportIndexEntry>,
    /// A mapping that associates the id of each re-export (`pub use ...`) to the id
    /// of the module it was re-exported from.
    pub re_export2parent_module: HashMap<rustdoc_types::Id, rustdoc_types::Id>,
}

/// An entry in [`ImportIndex`].
#[derive(
    Debug, Clone, Default, serde::Serialize, serde::Deserialize, bincode::Encode, bincode::Decode,
)]
pub struct ImportIndexEntry {
    /// All the public paths that can be used to import the item.
    pub public_paths: BTreeSet<SortablePath>,
    /// All the private paths that can be used to import the item.
    pub private_paths: BTreeSet<SortablePath>,
    /// The path where the item was originally defined.
    ///
    /// It may be set to `None` if we can't access the original definition.
    /// E.g. an item defined in a private module of `std`, where we only have access
    /// to the public API.
    pub defined_at: Option<Vec<String>>,
}

/// The visibility of a path inside [`ImportIndexEntry`].
pub enum EntryVisibility {
    /// The item can be imported from outside the crate where it was defined.
    Public,
    /// The item can only be imported from within the crate where it was defined.
    Private,
}

impl ImportIndexEntry {
    /// A private constructor.
    pub fn empty() -> Self {
        Self {
            public_paths: BTreeSet::new(),
            private_paths: BTreeSet::new(),
            defined_at: None,
        }
    }

    /// Create a new entry from a path.
    pub fn new(path: Vec<String>, visibility: EntryVisibility, is_definition: bool) -> Self {
        let mut entry = Self::empty();
        if is_definition {
            entry.defined_at = Some(path.clone());
        }
        match visibility {
            EntryVisibility::Public => entry.public_paths.insert(SortablePath(path)),
            EntryVisibility::Private => entry.private_paths.insert(SortablePath(path)),
        };
        entry
    }

    /// Add a new private path for this item.
    pub fn insert_private(&mut self, path: Vec<String>) {
        self.private_paths.insert(SortablePath(path));
    }

    /// Add a new path for this item.
    pub fn insert(&mut self, path: Vec<String>, visibility: EntryVisibility) {
        match visibility {
            EntryVisibility::Public => self.public_paths.insert(SortablePath(path)),
            EntryVisibility::Private => self.private_paths.insert(SortablePath(path)),
        };
    }

    /// Types can be exposed under multiple paths.
    /// This method returns a "canonical" importable path—i.e. the shortest importable path
    /// pointing at the type you specified.
    ///
    /// If the type is public, this method returns the shortest public path.
    /// If the type is private, this method returns the shortest private path.
    pub fn canonical_path(&self) -> &[String] {
        if let Some(SortablePath(p)) = self.public_paths.first() {
            return p;
        }
        if let Some(SortablePath(p)) = self.private_paths.first() {
            return p;
        }
        unreachable!("There must be at least one path associated to an import index entry")
    }

    /// Returns all paths associated with the type, both public and private.
    pub fn paths(&self) -> impl Iterator<Item = &[String]> {
        self.public_paths
            .iter()
            .map(|SortablePath(p)| p.as_slice())
            .chain(
                self.private_paths
                    .iter()
                    .map(|SortablePath(p)| p.as_slice()),
            )
    }
}

#[derive(
    Debug,
    Clone,
    Eq,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
    bincode::Encode,
    bincode::Decode,
)]
#[serde(transparent)]
pub struct SortablePath(pub Vec<String>);

impl Ord for SortablePath {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.0.len().cmp(&other.0.len()) {
            // Compare lexicographically if lengths are equal
            Ordering::Equal => self.0.cmp(&other.0),
            other => other,
        }
    }
}

impl PartialOrd for SortablePath {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
