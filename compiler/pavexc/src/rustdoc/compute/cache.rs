//! Thin wrapper around `pavexc_rustdoc_cache` that integrates with pavexc's types.

use std::borrow::Cow;

use guppy::PackageId;
use guppy::graph::PackageGraph;
use rkyv::rancor::Panic;
use rkyv::util::AlignedVec;

pub use pavexc_rustdoc_cache::{
    CacheEntry, EagerCrateItemIndex, EagerCrateItemPaths, EagerImportPath2Id, RkyvCowBytes,
    RustdocCacheKey, RustdocGlobalFsCache, SecondaryIndexes,
};
use pavexc_rustdoc_cache::HydratedCacheEntry as CacheEntryInner;

use crate::DiagnosticSink;
use crate::rustdoc::queries::CrateCore;

/// Extension trait to create `CacheEntry` from `&Crate`.
pub trait CacheEntryExt<'a> {
    /// Create a cache entry from a crate, including secondary indexes.
    fn from_crate(krate: &'a crate::rustdoc::Crate) -> Result<CacheEntry<'a>, anyhow::Error>;
    /// Create a raw cache entry from a crate (no secondary indexes).
    fn from_crate_raw(krate: &'a crate::rustdoc::Crate) -> Result<CacheEntry<'a>, anyhow::Error>;
}

impl<'a> CacheEntryExt<'a> for CacheEntry<'a> {
    fn from_crate(krate: &'a crate::rustdoc::Crate) -> Result<CacheEntry<'a>, anyhow::Error> {
        // Serialize the crate data
        let external_crates = bincode::serde::encode_to_vec(
            &krate.core.krate.external_crates,
            bincode::config::standard(),
        )?;

        // Serialize paths - handle Eager variant
        let paths: AlignedVec = match &krate.core.krate.paths {
            pavexc_rustdoc_cache::CrateItemPaths::Eager(EagerCrateItemPaths { paths }) => {
                rkyv::to_bytes::<Panic>(paths)?
            }
            pavexc_rustdoc_cache::CrateItemPaths::Lazy(lazy) => lazy.bytes.clone(),
        };

        // Serialize items - handle Eager variant
        let items: AlignedVec = match &krate.core.krate.index {
            pavexc_rustdoc_cache::CrateItemIndex::Eager(EagerCrateItemIndex { index }) => {
                rkyv::to_bytes::<Panic>(index)?
            }
            pavexc_rustdoc_cache::CrateItemIndex::Lazy(lazy) => lazy.bytes.clone(),
        };

        // Serialize import_path2id
        let import_path2id: AlignedVec = match &krate.import_path2id {
            pavexc_rustdoc_cache::ImportPath2Id::Eager(EagerImportPath2Id(m)) => {
                rkyv::to_bytes::<Panic>(m)?
            }
            pavexc_rustdoc_cache::ImportPath2Id::Lazy(lazy) => lazy.0.clone(),
        };

        // Serialize other secondary indexes
        let import_index =
            bincode::serde::encode_to_vec(&krate.import_index, bincode::config::standard())?;
        let annotated_items =
            bincode::serde::encode_to_vec(&krate.annotated_items, bincode::config::standard())?;
        let re_exports = bincode::serde::encode_to_vec(
            &krate.external_re_exports,
            bincode::config::standard(),
        )?;

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
            pavexc_rustdoc_cache::CrateItemPaths::Eager(EagerCrateItemPaths { paths }) => {
                rkyv::to_bytes::<Panic>(paths)?
            }
            pavexc_rustdoc_cache::CrateItemPaths::Lazy(lazy) => lazy.bytes.clone(),
        };

        // Serialize items - handle Eager variant
        let items: AlignedVec = match &krate.core.krate.index {
            pavexc_rustdoc_cache::CrateItemIndex::Eager(EagerCrateItemIndex { index }) => {
                rkyv::to_bytes::<Panic>(index)?
            }
            pavexc_rustdoc_cache::CrateItemIndex::Lazy(lazy) => lazy.bytes.clone(),
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

/// An entry retrieved from the on-disk cache.
pub(crate) enum RustdocCacheEntry {
    /// Only the "raw" output returned by `rustdoc` was stored in the cache.
    ///
    /// This happens when the indexing phase emitted one or more diagnostics,
    /// thus forcing to go through that step (and report those errors)
    /// every single time we attempt a compilation.
    Raw(CacheEntryInner),
    /// The cache holds both the raw `rustdoc` output and our secondary indexes.
    /// It's ready to be used as is!
    Processed(crate::rustdoc::Crate),
}

impl RustdocCacheEntry {
    /// Convert a cache entry from the rustdoc_cache crate to our internal representation.
    pub fn from_cache_inner(inner: CacheEntryInner) -> Self {
        match inner {
            CacheEntryInner::Raw(crate_data) => RustdocCacheEntry::Raw(CacheEntryInner::Raw(crate_data)),
            CacheEntryInner::Processed(processed) => {
                let krate = crate::rustdoc::Crate {
                    core: CrateCore {
                        package_id: processed.package_id,
                        krate: processed.crate_data,
                    },
                    import_path2id: processed.import_path2id,
                    import_index: processed.import_index,
                    external_re_exports: processed.external_re_exports,
                    annotated_items: processed.annotated_items,
                    crate_id2package_id: Default::default(),
                };
                RustdocCacheEntry::Processed(krate)
            }
        }
    }

    pub fn process(self, package_id: PackageId, sink: &DiagnosticSink) -> crate::rustdoc::Crate {
        match self {
            RustdocCacheEntry::Raw(inner) => {
                match inner {
                    CacheEntryInner::Raw(crate_data) => {
                        crate::rustdoc::Crate::index(crate_data, package_id, sink)
                    }
                    CacheEntryInner::Processed(processed) => {
                        // This shouldn't happen since we check above, but handle it gracefully
                        crate::rustdoc::Crate {
                            core: CrateCore {
                                package_id: processed.package_id,
                                krate: processed.crate_data,
                            },
                            import_path2id: processed.import_path2id,
                            import_index: processed.import_index,
                            external_re_exports: processed.external_re_exports,
                            annotated_items: processed.annotated_items,
                            crate_id2package_id: Default::default(),
                        }
                    }
                }
            }
            RustdocCacheEntry::Processed(c) => c,
        }
    }
}

/// Wrapper around [`RustdocGlobalFsCache`] that integrates with pavexc's caching fingerprint.
pub(crate) struct PavexRustdocCache {
    inner: RustdocGlobalFsCache,
}

impl std::fmt::Debug for PavexRustdocCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PavexRustdocCache")
            .field("inner", &self.inner)
            .finish()
    }
}

impl Clone for PavexRustdocCache {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl PavexRustdocCache {
    /// Construct the cache fingerprint for pavexc.
    fn cache_fingerprint() -> String {
        format!(
            "{}-{}",
            pavexc_rustdoc_cache::CRATE_VERSION,
            env!("RUSTDOC_CACHE_SOURCE_HASH")
        )
    }

    /// Initialize a new instance of the cache.
    pub(crate) fn new(
        toolchain_name: &str,
        cache_workspace_package_docs: bool,
        package_graph: &PackageGraph,
    ) -> Result<Self, anyhow::Error> {
        let fingerprint = Self::cache_fingerprint();
        let inner = RustdocGlobalFsCache::new(
            &fingerprint,
            toolchain_name,
            cache_workspace_package_docs,
            package_graph,
        )?;
        Ok(Self { inner })
    }

    /// Retrieve the cached documentation for a given package, if available.
    pub(crate) fn get(
        &self,
        cache_key: &RustdocCacheKey,
        package_graph: &PackageGraph,
    ) -> Result<Option<RustdocCacheEntry>, anyhow::Error> {
        match self.inner.get(cache_key, package_graph)? {
            Some(entry) => Ok(Some(RustdocCacheEntry::from_cache_inner(entry))),
            None => Ok(None),
        }
    }

    /// Store the JSON documentation for a crate in the cache.
    pub(crate) fn insert(
        &self,
        cache_key: &RustdocCacheKey,
        cache_entry: CacheEntry,
        package_graph: &PackageGraph,
    ) -> Result<(), anyhow::Error> {
        self.inner.insert(cache_key, cache_entry, package_graph)
    }

    /// Convert the JSON documentation generated by `rustdoc` into the format used by our cache,
    /// then store it.
    pub(crate) fn convert_and_insert(
        &self,
        cache_key: &RustdocCacheKey,
        krate: &crate::rustdoc::Crate,
        cache_indexes: bool,
        package_graph: &PackageGraph,
    ) -> Result<(), anyhow::Error> {
        let cache_entry = if cache_indexes {
            <CacheEntry as CacheEntryExt>::from_crate(krate)
        } else {
            <CacheEntry as CacheEntryExt>::from_crate_raw(krate)
        }?;
        self.insert(cache_key, cache_entry, package_graph)
    }

    /// Persist the list of package IDs that were accessed during the processing of the
    /// application blueprint for this project.
    pub(crate) fn persist_access_log(
        &self,
        package_ids: &std::collections::BTreeSet<PackageId>,
        project_fingerprint: &str,
    ) -> Result<(), anyhow::Error> {
        self.inner.persist_access_log(package_ids, project_fingerprint)
    }

    /// Retrieve the list of package IDs that were accessed during the last time we processed the
    /// application blueprint for this project.
    pub(crate) fn get_access_log(
        &self,
        project_fingerprint: &str,
    ) -> Result<std::collections::BTreeSet<PackageId>, anyhow::Error> {
        self.inner.get_access_log(project_fingerprint)
    }
}
