//! Index of all items in a crate.

use std::borrow::Cow;

use rkyv::collections::swiss_table::ArchivedHashMap;
use rkyv::rancor::Panic;
use rkyv::util::AlignedVec;
use rustc_hash::FxHashMap;
use rustdoc_types::{ArchivedId, ArchivedItem, Item};

/// The index of all the items in the crate.
///
/// Since the index can be quite large, we try to avoid deserializing it all at once.
///
/// The `Eager` variant contains the entire index, fully deserialized. This is what we get
/// when we have had to compute the documentation for the crate on the fly.
///
/// The `Lazy` variant contains the index as a byte array. There is a mapping from the
/// id of an item to the start and end index of the item's bytes in the byte array.
/// We can therefore deserialize the item only if we need to access it.
/// Since we only access a tiny portion of the items in the index (especially for large crates),
/// this translates in a significant performance improvement.
#[derive(Debug, Clone)]
pub enum CrateItemIndex {
    Eager(EagerCrateItemIndex),
    Lazy(LazyCrateItemIndex),
}

impl CrateItemIndex {
    /// Retrieve an item from the index given its id.
    pub fn get(&self, id: &rustdoc_types::Id) -> Option<Cow<'_, Item>> {
        match self {
            Self::Eager(index) => index.index.get(id).map(Cow::Borrowed),
            Self::Lazy(index) => {
                let item = index.get_deserialized(id)?;
                Some(Cow::Owned(item))
            }
        }
    }
}

/// See [`CrateItemIndex`] for more information.
#[derive(Debug, Clone)]
pub struct EagerCrateItemIndex {
    #[allow(clippy::disallowed_types)]
    pub index: FxHashMap<rustdoc_types::Id, Item>,
}

/// See [`CrateItemIndex`] for more information.
///
/// Stores rkyv-serialized bytes of a `HashMap<Id, Item>` and provides zero-copy access.
#[derive(Debug, Clone)]
pub struct LazyCrateItemIndex {
    /// The rkyv-serialized bytes containing a `HashMap<Id, Item>`.
    pub bytes: AlignedVec,
}

impl LazyCrateItemIndex {
    /// Get zero-copy access to the archived HashMap.
    #[inline]
    fn archived(&self) -> &ArchivedHashMap<ArchivedId, ArchivedItem> {
        // SAFETY: The bytes were serialized by rkyv from a valid HashMap<Id, Item>.
        // We trust the cache to contain valid data.
        unsafe { rkyv::access_unchecked::<ArchivedHashMap<ArchivedId, ArchivedItem>>(&self.bytes) }
    }

    /// Get an item by its ID, returning a reference to the archived item.
    pub fn get(&self, id: &rustdoc_types::Id) -> Option<&ArchivedItem> {
        self.archived().get(&ArchivedId(id.0.into()))
    }

    /// Deserialize an item by its ID.
    pub fn get_deserialized(&self, id: &rustdoc_types::Id) -> Option<Item> {
        let archived = self.get(id)?;
        Some(rkyv::deserialize::<Item, Panic>(archived).unwrap())
    }
}
