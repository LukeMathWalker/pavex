//! Mapping from import paths to item IDs.

use ahash::HashMap;
use rkyv::collections::swiss_table::ArchivedHashMap;
use rkyv::rancor::Panic;
use rkyv::string::ArchivedString;
use rkyv::util::AlignedVec;
use rkyv::vec::ArchivedVec;
use rustdoc_types::ArchivedId;

/// A mapping from import paths to the id of the item they point to.
///
/// The `Eager` variant contains the entire mapping, fully deserialized.
///
/// The `Lazy` variant contains the index as a byte array, with entries deserialized on demand.
#[derive(Debug, Clone)]
pub enum ImportPath2Id {
    Eager(EagerImportPath2Id),
    Lazy(LazyImportPath2Id),
}

impl ImportPath2Id {
    pub fn get(&self, path: &[String]) -> Option<rustdoc_types::Id> {
        match self {
            ImportPath2Id::Eager(m) => m.0.get(path).cloned(),
            ImportPath2Id::Lazy(m) => m.get_deserialized(path),
        }
    }
}

/// See [`ImportPath2Id`] for more information.
#[derive(Debug, Clone)]
pub struct EagerImportPath2Id(pub HashMap<Vec<String>, rustdoc_types::Id>);

/// See [`ImportPath2Id`] for more information.
///
/// Stores rkyv-serialized bytes of a `HashMap<Vec<String>, Id>` and provides zero-copy access.
#[derive(Debug, Clone)]
pub struct LazyImportPath2Id(pub AlignedVec);

impl LazyImportPath2Id {
    #[inline]
    fn archived(&self) -> &ArchivedHashMap<ArchivedVec<ArchivedString>, ArchivedId> {
        unsafe {
            rkyv::access_unchecked::<ArchivedHashMap<ArchivedVec<ArchivedString>, ArchivedId>>(
                &self.0,
            )
        }
    }

    pub fn get(&self, path: &[String]) -> Option<&ArchivedId> {
        let path_vec: Vec<String> = path.to_vec();
        let bytes = rkyv::to_bytes::<Panic>(&path_vec).ok()?;

        let archived_key = unsafe { rkyv::access_unchecked::<ArchivedVec<ArchivedString>>(&bytes) };
        self.archived().get(archived_key)
    }

    pub fn get_deserialized(&self, path: &[String]) -> Option<rustdoc_types::Id> {
        let archived = self.get(path)?;
        Some(rkyv::deserialize::<_, Panic>(archived).unwrap())
    }
}
