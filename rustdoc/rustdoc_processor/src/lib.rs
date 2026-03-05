//! Compute, cache, index, and query `rustdoc` JSON documentation for crates
//! in a project's dependency graph.
//!
//! The pipeline:
//!
//! 1. **Compute** ([`compute_crate_docs`]): invoke `cargo rustdoc` to generate JSON docs.
//! 2. **Cache** ([`RustdocGlobalFsCache`]): persist raw docs and secondary indexes
//!    in a SQLite database at `{cache_dir}/{fingerprint}.db`.
//! 3. **Index** ([`index_local_types`]): build secondary indexes — import paths, item lookups,
//!    and external re-export tracking.
//! 4. **Query** ([`Crate`], [`CrateRegistry`]): look up items by path, resolve
//!    cross-crate references, and retrieve canonical import paths.

mod cache;
mod compute;
mod crate_data;
mod global_item_id;
mod indexing;
mod queries;
mod unknown_item_path;
mod utils;
mod version_matcher;

// Cache
pub use cache::entry::{CacheEntry, SecondaryIndexes};
pub use cache::utils::RkyvCowBytes;
pub use cache::{HydratedCacheEntry, ProcessedCacheEntry, RustdocCacheKey, RustdocGlobalFsCache};

// Compute
pub use compute::{CannotGetCrateData, ComputeProgress, NoProgress, compute_crate_docs};

// Crate data model
pub use crate_data::{CrateData, CrateItemIndex, EagerCrateItemIndex};
pub use crate_data::{CrateItemPaths, EagerCrateItemPaths};

// Indexing
pub use indexing::ExternalReExports;
pub use indexing::ImportIndex;
pub use indexing::IndexingVisitor;
pub use indexing::{EagerImportPath2Id, ImportPath2Id};

// Queries
pub use queries::{Crate, CrateCore, CrateRegistry};

// Cross-cutting
pub use global_item_id::GlobalItemId;
pub use unknown_item_path::UnknownItemPath;

/// Crate version - used as part of cache fingerprint.
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Standard library crate package ID representation.
pub const STD_PACKAGE_ID_REPR: &str = "std";
/// Core crate package ID representation.
pub const CORE_PACKAGE_ID_REPR: &str = "core";
/// Alloc crate package ID representation.
pub const ALLOC_PACKAGE_ID_REPR: &str = "alloc";

/// The set of toolchain crates that are bundled with Rust.
pub const TOOLCHAIN_CRATES: [&str; 3] = [
    STD_PACKAGE_ID_REPR,
    CORE_PACKAGE_ID_REPR,
    ALLOC_PACKAGE_ID_REPR,
];

/// Return the options to pass to `rustdoc` in order to generate JSON documentation.
///
/// We isolate this logic in a separate function in order to be able to refer to these
/// options from various places in the codebase and maintain a single source of truth.
///
/// In particular, they do affect our caching logic (see the `cache` module).
pub(crate) fn rustdoc_options() -> [&'static str; 4] {
    [
        "--document-private-items",
        "-Zunstable-options",
        "-wjson",
        "--document-hidden-items",
    ]
}
