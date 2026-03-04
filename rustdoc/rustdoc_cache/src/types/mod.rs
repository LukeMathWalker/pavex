//! Types for storing and retrieving rustdoc JSON documentation in the cache.

mod cache_entry;
mod crate_data;
mod global_item_id;
mod import_index;
mod import_path;
mod item_index;
mod item_paths;
mod re_exports;
mod indexing;
pub(crate) mod krate;
mod krate_core;
mod registry;
mod resolution;
mod unknown_item_path;

pub use cache_entry::{CacheEntry, RkyvCowBytes, SecondaryIndexes, ThirdPartyCrateCacheKey};
pub use crate_data::CrateData;
pub use krate_core::CrateCore;
pub use global_item_id::GlobalItemId;
pub use indexing::{IndexingVisitor, NoopVisitor, index_local_types};
pub use import_index::{EntryVisibility, ImportIndex, ImportIndexEntry, SortablePath};
pub use import_path::{EagerImportPath2Id, ImportPath2Id, LazyImportPath2Id};
pub use item_index::{CrateItemIndex, EagerCrateItemIndex, LazyCrateItemIndex};
pub use item_paths::{
    CrateItemPaths, CrateItemPathsIter, EagerCrateItemPaths, ItemSummaryRef, LazyCrateItemPaths,
};
pub use re_exports::{ExternalReExport, ExternalReExports};
pub use krate::Crate;
pub use registry::CrateRegistry;
pub use resolution::{CrateIdNeedle, compute_package_id_for_crate_id};
pub use unknown_item_path::UnknownItemPath;
