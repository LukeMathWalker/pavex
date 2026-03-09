//! Thin wrapper around `rustdoc_processor` that integrates with pavexc's types.

use std::borrow::Cow;

use guppy::graph::PackageGraph;
use rkyv::rancor::Panic;
use rkyv::util::AlignedVec;

pub use rustdoc_processor::cache::{
    CacheEntry, RkyvCowBytes, RustdocCacheKey, RustdocGlobalFsCache, SecondaryIndexes,
};
pub use rustdoc_processor::crate_data::{EagerCrateItemIndex, EagerCrateItemPaths};
pub use rustdoc_processor::indexing::EagerImportPath2Id;

/// Extension trait to create `CacheEntry` from `&Crate`.
pub trait CacheEntryExt<'a> {
    /// Create a cache entry from a crate, including secondary indexes.
    fn from_crate<A: serde::Serialize>(
        krate: &'a crate::rustdoc::Crate,
        annotations: &'a A,
    ) -> Result<CacheEntry<'a>, anyhow::Error>;
    /// Create a raw cache entry from a crate (no secondary indexes).
    fn from_crate_raw(krate: &'a crate::rustdoc::Crate) -> Result<CacheEntry<'a>, anyhow::Error>;
}

impl<'a> CacheEntryExt<'a> for CacheEntry<'a> {
    fn from_crate<A: serde::Serialize>(
        krate: &'a crate::rustdoc::Crate,
        annotations: &'a A,
    ) -> Result<CacheEntry<'a>, anyhow::Error> {
        // Serialize the crate data
        let external_crates = bincode::serde::encode_to_vec(
            &krate.core.krate.external_crates,
            bincode::config::standard(),
        )?;

        // Serialize paths - handle Eager variant
        let paths: AlignedVec = match &krate.core.krate.paths {
            rustdoc_processor::crate_data::CrateItemPaths::Eager(EagerCrateItemPaths { paths }) => {
                rkyv::to_bytes::<Panic>(paths)?
            }
            rustdoc_processor::crate_data::CrateItemPaths::Lazy(lazy) => lazy.bytes.clone(),
        };

        // Serialize items - handle Eager variant
        let items: AlignedVec = match &krate.core.krate.index {
            rustdoc_processor::crate_data::CrateItemIndex::Eager(EagerCrateItemIndex { index }) => {
                rkyv::to_bytes::<Panic>(index)?
            }
            rustdoc_processor::crate_data::CrateItemIndex::Lazy(lazy) => lazy.bytes.clone(),
        };

        // Serialize import_path2id
        let import_path2id: AlignedVec = match &krate.import_path2id {
            rustdoc_processor::indexing::ImportPath2Id::Eager(EagerImportPath2Id(m)) => {
                rkyv::to_bytes::<Panic>(m)?
            }
            rustdoc_processor::indexing::ImportPath2Id::Lazy(lazy) => lazy.0.clone(),
        };

        // Serialize other secondary indexes
        let import_index =
            bincode::serde::encode_to_vec(&krate.import_index, bincode::config::standard())?;
        let annotated_items =
            bincode::serde::encode_to_vec(annotations, bincode::config::standard())?;
        let re_exports =
            bincode::serde::encode_to_vec(&krate.external_re_exports, bincode::config::standard())?;

        let secondary_indexes = SecondaryIndexes {
            import_index: Cow::Owned(import_index),
            annotated_items: Some(Cow::Owned(annotated_items)),
            import_path2id: RkyvCowBytes::Owned(import_path2id),
            re_exports: Cow::Owned(re_exports),
        };

        Ok(CacheEntry {
            root_item_id: krate.core.krate.root_item_id.0,
            external_crates: Cow::Owned(external_crates),
            paths: RkyvCowBytes::Owned(paths),
            format_version: krate.core.krate.format_version as i64,
            items: RkyvCowBytes::Owned(items),
            secondary_indexes: Some(secondary_indexes),
        })
    }

    fn from_crate_raw(krate: &'a crate::rustdoc::Crate) -> Result<CacheEntry<'a>, anyhow::Error> {
        // Serialize the crate data
        let external_crates = bincode::serde::encode_to_vec(
            &krate.core.krate.external_crates,
            bincode::config::standard(),
        )?;

        // Serialize paths - handle Eager variant
        let paths: AlignedVec = match &krate.core.krate.paths {
            rustdoc_processor::crate_data::CrateItemPaths::Eager(EagerCrateItemPaths { paths }) => {
                rkyv::to_bytes::<Panic>(paths)?
            }
            rustdoc_processor::crate_data::CrateItemPaths::Lazy(lazy) => lazy.bytes.clone(),
        };

        // Serialize items - handle Eager variant
        let items: AlignedVec = match &krate.core.krate.index {
            rustdoc_processor::crate_data::CrateItemIndex::Eager(EagerCrateItemIndex { index }) => {
                rkyv::to_bytes::<Panic>(index)?
            }
            rustdoc_processor::crate_data::CrateItemIndex::Lazy(lazy) => lazy.bytes.clone(),
        };

        Ok(CacheEntry {
            root_item_id: krate.core.krate.root_item_id.0,
            external_crates: Cow::Owned(external_crates),
            paths: RkyvCowBytes::Owned(paths),
            format_version: krate.core.krate.format_version as i64,
            items: RkyvCowBytes::Owned(items),
            secondary_indexes: None,
        })
    }
}

/// Construct a [`RustdocGlobalFsCache`] pre-configured with Pavex's cache fingerprint
/// and default cache directory (`~/.pavex/rustdoc/cache`).
pub(crate) fn pavex_rustdoc_cache<A: Default + bincode::Decode<()>>(
    toolchain_name: &str,
    cache_workspace_package_docs: bool,
    package_graph: &PackageGraph,
) -> Result<RustdocGlobalFsCache<A>, anyhow::Error> {
    let fingerprint = format!(
        "{}-{}",
        rustdoc_processor::CRATE_VERSION,
        env!("RUSTDOC_CACHE_SOURCE_HASH")
    );
    let cache_dir = xdg_home::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Failed to get the user's home directory"))?
        .join(".pavex/rustdoc/cache");
    RustdocGlobalFsCache::new(
        &fingerprint,
        toolchain_name,
        cache_workspace_package_docs,
        package_graph,
        &cache_dir,
    )
}
