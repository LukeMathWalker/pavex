//! Cache for toolchain crates (std, core, alloc).

use std::borrow::Cow;

use guppy::PackageId;
use rusqlite::params;
use tracing::instrument;

use crate::annotations::AnnotatedItems;
use crate::types::{
    CacheEntry, CrateData, CrateItemIndex, CrateItemPaths, ImportPath2Id, LazyCrateItemIndex,
    LazyCrateItemPaths, LazyImportPath2Id, RkyvCowBytes, SecondaryIndexes,
};

use super::{ProcessedCacheEntry, HydratedCacheEntry, BINCODE_CONFIG};

#[derive(Debug, Clone)]
#[non_exhaustive]
pub(super) struct ToolchainCache {}

impl ToolchainCache {
    pub(super) fn new(connection: &rusqlite::Connection) -> Result<Self, anyhow::Error> {
        Self::setup_table(connection)?;
        Ok(Self {})
    }

    /// Retrieve the cached documentation for a given toolchain crate, if available.
    #[instrument(name = "Retrieve cached toolchain docs from disk",
        skip_all,
        level=tracing::Level::DEBUG,
        fields(crate.name = %name)
    )]
    pub(super) fn get(
        &self,
        name: &str,
        cargo_fingerprint: &str,
        connection: &rusqlite::Connection,
    ) -> Result<Option<HydratedCacheEntry>, anyhow::Error> {
        // Retrieve from rustdoc's output from cache, if available.
        let mut stmt = connection.prepare_cached(
            "SELECT
                root_item_id,
                external_crates,
                paths,
                format_version,
                items,
                import_index,
                import_path2id,
                re_exports
            FROM rustdoc_toolchain_crates_cache
            WHERE name = ? AND cargo_fingerprint = ?",
        )?;

        let span = tracing::trace_span!("Execute query");
        let guard = span.enter();
        let mut rows = stmt.query(params![name, cargo_fingerprint])?;
        let Some(row) = rows.next()? else {
            return Ok(None);
        };
        drop(guard);

        let root_item_id = row.get_ref_unwrap(0).as_i64()?.try_into()?;
        let external_crates = row.get_ref_unwrap(1).as_bytes()?;
        let paths = row.get_ref_unwrap(2).as_bytes()?;
        let format_version = row.get_ref_unwrap(3).as_i64()?;

        let items = row.get_ref_unwrap(4).as_bytes()?;

        let import_index = row.get_ref_unwrap(5).as_bytes()?;
        let import_path2id = row.get_ref_unwrap(6).as_bytes()?;
        let re_exports = row.get_ref_unwrap(7).as_bytes()?;

        let krate = CacheEntry {
            root_item_id,
            external_crates: Cow::Borrowed(external_crates),
            paths: RkyvCowBytes::Borrowed(paths),
            format_version,
            items: RkyvCowBytes::Borrowed(items),
            secondary_indexes: Some(SecondaryIndexes {
                import_index: Cow::Borrowed(import_index),
                // Standard library crates don't have Pavex annotations.
                annotated_items: None,
                import_path2id: RkyvCowBytes::Borrowed(import_path2id),
                re_exports: Cow::Borrowed(re_exports),
            }),
        }
        .hydrate(PackageId::new(name))?;

        Ok(Some(krate))
    }

    /// Store the JSON documentation for a toolchain crate in the cache.
    #[instrument(name = "Cache rustdoc output on disk", skip_all, level=tracing::Level::DEBUG, fields(crate.name = name))]
    pub(super) fn insert(
        &self,
        name: &str,
        cache_entry: CacheEntry<'_>,
        cargo_fingerprint: &str,
        connection: &rusqlite::Connection,
    ) -> Result<(), anyhow::Error> {
        let mut stmt = connection.prepare_cached(
            "INSERT INTO rustdoc_toolchain_crates_cache (
                name,
                cargo_fingerprint,
                root_item_id,
                external_crates,
                paths,
                format_version,
                items,
                import_index,
                import_path2id,
                re_exports
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )?;
        stmt.execute(params![
            name,
            cargo_fingerprint,
            cache_entry.root_item_id,
            cache_entry.external_crates,
            cache_entry.paths,
            cache_entry.format_version,
            cache_entry.items,
            cache_entry
                .secondary_indexes
                .as_ref()
                .expect("Indexing never fails for toolchain crates")
                .import_index,
            cache_entry
                .secondary_indexes
                .as_ref()
                .expect("Indexing never fails for toolchain crates")
                .import_path2id,
            cache_entry
                .secondary_indexes
                .as_ref()
                .expect("Indexing never fails for toolchain crates")
                .re_exports
        ])?;
        Ok(())
    }

    fn setup_table(connection: &rusqlite::Connection) -> Result<(), anyhow::Error> {
        connection.execute(
            "CREATE TABLE IF NOT EXISTS rustdoc_toolchain_crates_cache (
                name TEXT NOT NULL,
                cargo_fingerprint TEXT NOT NULL,
                root_item_id INTEGER NOT NULL,
                external_crates BLOB NOT NULL,
                paths BLOB NOT NULL,
                format_version INTEGER NOT NULL,
                items BLOB NOT NULL,
                import_index BLOB NOT NULL,
                import_path2id BLOB NOT NULL,
                re_exports BLOB NOT NULL,
                PRIMARY KEY (name, cargo_fingerprint)
            )",
            [],
        )?;
        Ok(())
    }
}

impl CacheEntry<'_> {
    /// Re-hydrate the documentation retrieved from the cache.
    ///
    /// We hydrate all mappings eagerly, but we avoid re-hydrating the item index eagerly,
    /// since it can be quite large and deserialization can be slow for large crates.
    /// The item index is stored as rkyv-serialized bytes for zero-copy access.
    pub fn hydrate(self, package_id: PackageId) -> Result<HydratedCacheEntry, anyhow::Error> {
        use anyhow::Context;

        let crate_data = CrateData {
            root_item_id: rustdoc_types::Id(self.root_item_id.to_owned()),
            external_crates: bincode::decode_from_slice(&self.external_crates, BINCODE_CONFIG)
                .context("Failed to deserialize external_crates")?
                .0,
            paths: CrateItemPaths::Lazy(LazyCrateItemPaths {
                bytes: self.paths.into_owned(),
            }),
            format_version: self.format_version.try_into()?,
            index: CrateItemIndex::Lazy(LazyCrateItemIndex {
                bytes: self.items.into_owned(),
            }),
        };
        let Some(secondary_indexes) = self.secondary_indexes else {
            return Ok(HydratedCacheEntry::Raw(crate_data));
        };

        let import_index =
            bincode::decode_from_slice(&secondary_indexes.import_index, BINCODE_CONFIG)
                .context("Failed to deserialize import_index")?
                .0;

        let re_exports = bincode::decode_from_slice(&secondary_indexes.re_exports, BINCODE_CONFIG)
            .context("Failed to deserialize re-exports")?
            .0;

        let annotated_items = if let Some(data) = secondary_indexes.annotated_items {
            bincode::decode_from_slice(&data, BINCODE_CONFIG)
                .context("Failed to deserialize annotated_items")?
                .0
        } else {
            AnnotatedItems::default()
        };

        let processed = ProcessedCacheEntry {
            package_id,
            crate_data,
            import_path2id: ImportPath2Id::Lazy(LazyImportPath2Id(
                secondary_indexes.import_path2id.into_owned(),
            )),
            external_re_exports: re_exports,
            import_index,
            annotated_items,
        };
        Ok(HydratedCacheEntry::Processed(processed))
    }
}
