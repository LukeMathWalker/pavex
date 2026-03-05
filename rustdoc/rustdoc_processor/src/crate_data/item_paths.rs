//! Mapping from item IDs to their paths.

use std::borrow::Cow;

use rkyv::collections::swiss_table::ArchivedHashMap;
use rkyv::hash::FxHasher64;
use rkyv::rancor::Panic;
use rkyv::util::AlignedVec;
use rustc_hash::FxHashMap;
use rustdoc_types::{ArchivedId, ArchivedItemSummary, ItemKind, ItemSummary};

/// A mapping from the id of a type to its fully qualified path.
///
/// Primarily useful for foreign items that are being re-exported by this crate.
#[derive(Debug, Clone)]
pub enum CrateItemPaths {
    Eager(EagerCrateItemPaths),
    Lazy(LazyCrateItemPaths),
}

impl CrateItemPaths {
    /// Retrieve an item summary from the index given its id.
    pub fn get(&self, id: &rustdoc_types::Id) -> Option<Cow<'_, ItemSummary>> {
        match self {
            Self::Eager(m) => m.paths.get(id).map(Cow::Borrowed),
            Self::Lazy(m) => {
                let item = m.get_deserialized(id)?;
                Some(Cow::Owned(item))
            }
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (rustdoc_types::Id, ItemSummaryRef<'_>)> {
        match self {
            CrateItemPaths::Eager(paths) => CrateItemPathsIter::Eager(paths.paths.iter()),
            CrateItemPaths::Lazy(paths) => CrateItemPathsIter::Lazy(paths.archived().iter()),
        }
    }
}

pub enum CrateItemPathsIter<'a> {
    Eager(std::collections::hash_map::Iter<'a, rustdoc_types::Id, ItemSummary>),
    Lazy(
        rkyv::collections::swiss_table::map::Iter<'a, ArchivedId, ArchivedItemSummary, FxHasher64>,
    ),
}

pub enum ItemSummaryRef<'a> {
    Eager(&'a ItemSummary),
    Lazy(&'a ArchivedItemSummary),
}

impl<'a> ItemSummaryRef<'a> {
    pub fn crate_id(&self) -> u32 {
        match self {
            ItemSummaryRef::Eager(s) => s.crate_id,
            ItemSummaryRef::Lazy(s) => s.crate_id.to_native(),
        }
    }

    pub fn kind(&self) -> ItemKind {
        match self {
            ItemSummaryRef::Eager(s) => s.kind,
            ItemSummaryRef::Lazy(s) => {
                // Safe to do since the enum is repr(u8)
                rkyv::deserialize::<_, rkyv::rancor::Infallible>(&s.kind).unwrap()
            }
        }
    }

    pub fn path(&self) -> Cow<'_, [String]> {
        match self {
            ItemSummaryRef::Eager(s) => Cow::Borrowed(&s.path),
            ItemSummaryRef::Lazy(s) => {
                Cow::Owned(s.path.iter().map(|s| s.as_str().to_owned()).collect())
            }
        }
    }
}

impl<'a> Iterator for CrateItemPathsIter<'a> {
    type Item = (rustdoc_types::Id, ItemSummaryRef<'a>);

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Eager(iter) => iter.next().map(|(k, v)| (*k, ItemSummaryRef::Eager(v))),
            Self::Lazy(iter) => iter
                .next()
                .map(|(k, v)| (rustdoc_types::Id(k.0.to_native()), ItemSummaryRef::Lazy(v))),
        }
    }
}

/// See [`CrateItemPaths`] for more information.
#[derive(Debug, Clone)]
pub struct EagerCrateItemPaths {
    #[allow(clippy::disallowed_types)]
    pub paths: FxHashMap<rustdoc_types::Id, ItemSummary>,
}

/// See [`CrateItemPaths`] for more information.
#[derive(Debug, Clone)]
pub struct LazyCrateItemPaths {
    pub bytes: AlignedVec,
}

impl LazyCrateItemPaths {
    /// Get zero-copy access to the archived HashMap.
    #[inline]
    fn archived(&self) -> &ArchivedHashMap<ArchivedId, ArchivedItemSummary> {
        // SAFETY: The bytes were serialized by rkyv from a valid HashMap<Id, ItemSummary>.
        // We trust the cache to contain valid data.
        unsafe {
            rkyv::access_unchecked::<ArchivedHashMap<ArchivedId, ArchivedItemSummary>>(&self.bytes)
        }
    }

    /// Get an item by its ID, returning a reference to the archived summary.
    pub fn get(&self, id: &rustdoc_types::Id) -> Option<&ArchivedItemSummary> {
        self.archived().get(&ArchivedId(id.0.into()))
    }

    /// Deserialize a summary by its ID.
    pub fn get_deserialized(&self, id: &rustdoc_types::Id) -> Option<ItemSummary> {
        let archived = self.get(id)?;
        Some(rkyv::deserialize::<ItemSummary, Panic>(archived).unwrap())
    }
}
