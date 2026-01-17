//! Cache for third-party crates.

use std::borrow::Cow;

use anyhow::Context;
use camino::Utf8Path;
use guppy::graph::feature::StandardFeatures;
use guppy::graph::{PackageGraph, PackageMetadata};
use rusqlite::params;
use tracing::instrument;
use tracing_log_error::log_error;

use crate::checksum::checksum_crate;
use crate::rustdoc_options;
use crate::types::{
    CacheEntry, RkyvCowBytes, SecondaryIndexes, ThirdPartyCrateCacheKey,
};

use super::HydratedCacheEntry;

#[derive(Debug, Clone)]
#[non_exhaustive]
pub(super) struct ThirdPartyCrateCache {
    pub(super) cache_workspace_packages: bool,
}

impl ThirdPartyCrateCache {
    pub(super) fn new(
        connection: &rusqlite::Connection,
        cache_workspace_packages: bool,
        package_graph: &PackageGraph,
    ) -> Result<Self, anyhow::Error> {
        Self::setup_table(connection)?;
        // Force the creation of the feature graph ahead of our queries.
        // It'll be cached internally by the `package_graph`.
        let _ = package_graph.feature_graph();
        Ok(Self {
            cache_workspace_packages,
        })
    }

    /// Retrieve the cached documentation for a given package, if available.
    #[instrument(name = "Retrieve third-party crate docs from disk cache",
        skip_all,
        level=tracing::Level::DEBUG,
        fields(crate.id = %package_metadata.id(), cache_key = tracing::field::Empty, hit = tracing::field::Empty)
    )]
    pub(super) fn get(
        &self,
        package_metadata: &PackageMetadata,
        cargo_fingerprint: &str,
        connection: &rusqlite::Connection,
        package_graph: &PackageGraph,
    ) -> Result<Option<HydratedCacheEntry>, anyhow::Error> {
        fn _get(
            package_metadata: &PackageMetadata,
            cargo_fingerprint: &str,
            connection: &rusqlite::Connection,
            cache_workspace_packages: bool,
            package_graph: &PackageGraph,
        ) -> Result<Option<HydratedCacheEntry>, anyhow::Error> {
            let Some(cache_key) = ThirdPartyCrateCacheKey::build(
                package_graph,
                package_metadata,
                cargo_fingerprint,
                cache_workspace_packages,
            ) else {
                return Ok(None);
            };
            tracing::Span::current().record("cache_key", tracing::field::debug(&cache_key));
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
                        re_exports,
                        annotated_items
                    FROM rustdoc_3d_party_crates_cache
                    WHERE crate_name = ? AND
                        crate_source = ? AND
                        crate_version = ? AND
                        crate_hash = ? AND
                        cargo_fingerprint = ? AND
                        rustdoc_options = ? AND
                        default_feature_is_enabled = ? AND
                        active_named_features = ?",
            )?;
            let span = tracing::trace_span!("Execute query");
            let guard = span.enter();
            let mut rows = stmt.query(params![
                cache_key.crate_name,
                cache_key.crate_source,
                cache_key.crate_version,
                // `NULL` values are considered to be distinct from all other values
                // by SQLite, including other `NULL`s. Therefore we use an empty
                // string as a placeholder for `NULL` values.
                cache_key.crate_hash.unwrap_or_default(),
                cache_key.cargo_fingerprint,
                cache_key.rustdoc_options,
                cache_key.default_feature_is_enabled,
                cache_key.active_named_features
            ])?;
            let Some(row) = rows.next().context("Failed to fetch next row")? else {
                return Ok(None);
            };
            drop(guard);

            let root_item_id = row.get_ref_unwrap(0).as_i64()?.try_into()?;
            let external_crates = row.get_ref_unwrap(1).as_bytes()?;
            let paths = row.get_ref_unwrap(2).as_bytes()?;
            let format_version = row.get_ref_unwrap(3).as_i64()?;
            let items = row.get_ref_unwrap(4).as_bytes()?;
            let import_index = row.get_ref_unwrap(5).as_bytes_or_null()?;
            let import_path2id = row.get_ref_unwrap(6).as_bytes_or_null()?;
            let re_exports = row.get_ref_unwrap(7).as_bytes_or_null()?;
            let annotated_items = row.get_ref_unwrap(8).as_bytes_or_null()?;

            let secondary_indexes =
                match (import_index, import_path2id, re_exports, annotated_items) {
                    (
                        Some(import_index),
                        Some(import_path2id),
                        Some(re_exports),
                        Some(annotated_items),
                    ) => Some(SecondaryIndexes {
                        import_index: Cow::Borrowed(import_index),
                        import_path2id: RkyvCowBytes::Borrowed(import_path2id),
                        re_exports: Cow::Borrowed(re_exports),
                        annotated_items: Some(Cow::Borrowed(annotated_items)),
                    }),
                    _ => None,
                };

            let krate = CacheEntry {
                root_item_id,
                external_crates: Cow::Borrowed(external_crates),
                paths: RkyvCowBytes::Borrowed(paths),
                format_version,
                items: RkyvCowBytes::Borrowed(items),
                secondary_indexes,
            }
            .hydrate(package_metadata.id().to_owned())
            .context("Failed to re-hydrate the stored docs")?;

            Ok(Some(krate))
        }
        let outcome = _get(
            package_metadata,
            cargo_fingerprint,
            connection,
            self.cache_workspace_packages,
            package_graph,
        );
        match &outcome {
            Ok(Some(_)) => {
                tracing::Span::current().record("hit", true);
            }
            Ok(None) => {
                tracing::Span::current().record("hit", false);
            }
            _ => {}
        }
        outcome
    }

    /// Compute the cache key for a given package.
    pub(super) fn cache_key<'a>(
        &self,
        package_metadata: &'a PackageMetadata,
        cargo_fingerprint: &'a str,
        package_graph: &PackageGraph,
    ) -> Option<ThirdPartyCrateCacheKey<'a>> {
        ThirdPartyCrateCacheKey::build(
            package_graph,
            package_metadata,
            cargo_fingerprint,
            self.cache_workspace_packages,
        )
    }

    /// Store the JSON documentation generated by `rustdoc` in the cache.
    #[instrument(
        name = "Stored cache data for third-party crate docs to disk",
        skip_all,
        level=tracing::Level::DEBUG,
        fields(cache_key = tracing::field::Empty))
    ]
    pub(super) fn insert(
        &self,
        cache_key: ThirdPartyCrateCacheKey<'_>,
        connection: &rusqlite::Connection,
        cached_data: CacheEntry<'_>,
    ) -> Result<(), anyhow::Error> {
        tracing::Span::current().record("cache_key", tracing::field::debug(&cache_key));
        let mut stmt = connection.prepare_cached(
            "INSERT INTO rustdoc_3d_party_crates_cache (
                crate_name,
                crate_source,
                crate_version,
                crate_hash,
                cargo_fingerprint,
                rustdoc_options,
                default_feature_is_enabled,
                active_named_features,
                root_item_id,
                external_crates,
                paths,
                format_version,
                items,
                import_index,
                import_path2id,
                re_exports,
                annotated_items
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )?;
        stmt.execute(params![
            cache_key.crate_name,
            cache_key.crate_source,
            cache_key.crate_version,
            // `NULL` values are considered to be distinct from all other values
            // by SQLite, including other `NULL`s. Therefore we use an empty
            // string as a placeholder for `NULL` values.
            cache_key.crate_hash.unwrap_or_default(),
            cache_key.cargo_fingerprint,
            cache_key.rustdoc_options,
            cache_key.default_feature_is_enabled,
            cache_key.active_named_features,
            cached_data.root_item_id,
            cached_data.external_crates,
            cached_data.paths,
            cached_data.format_version,
            cached_data.items,
            cached_data
                .secondary_indexes
                .as_ref()
                .map(|i| i.import_index.as_ref()),
            cached_data
                .secondary_indexes
                .as_ref()
                .map(|indexes| indexes.import_path2id.as_ref()),
            cached_data
                .secondary_indexes
                .as_ref()
                .map(|indexes| indexes.re_exports.as_ref()),
            cached_data
                .secondary_indexes
                .as_ref()
                .map(|indexes| indexes.annotated_items.as_ref())
        ])?;
        Ok(())
    }

    fn setup_table(connection: &rusqlite::Connection) -> Result<(), anyhow::Error> {
        connection.execute(
            "CREATE TABLE IF NOT EXISTS rustdoc_3d_party_crates_cache (
                crate_name TEXT NOT NULL,
                crate_source TEXT NOT NULL,
                crate_version TEXT NOT NULL,
                crate_hash TEXT NOT NULL,
                cargo_fingerprint TEXT NOT NULL,
                rustdoc_options TEXT NOT NULL,
                default_feature_is_enabled INTEGER NOT NULL,
                active_named_features TEXT NOT NULL,
                root_item_id INTEGER NOT NULL,
                external_crates BLOB NOT NULL,
                paths BLOB NOT NULL,
                format_version INTEGER NOT NULL,
                items BLOB NOT NULL,
                annotated_items BLOB,
                import_index BLOB,
                import_path2id BLOB,
                re_exports BLOB,
                PRIMARY KEY (crate_name, crate_source, crate_version, crate_hash, cargo_fingerprint, rustdoc_options, default_feature_is_enabled, active_named_features)
            )",
            []
        )?;
        Ok(())
    }
}

impl<'a> ThirdPartyCrateCacheKey<'a> {
    /// Compute the cache key for a given package.
    pub fn build(
        package_graph: &PackageGraph,
        package_metadata: &'a PackageMetadata<'a>,
        cargo_fingerprint: &'a str,
        cache_workspace_packages: bool,
    ) -> Option<ThirdPartyCrateCacheKey<'a>> {
        enum PathOrId<'a> {
            Path(Cow<'a, Utf8Path>),
            Id(&'a str),
        }

        impl<'a> From<PathOrId<'a>> for Cow<'a, str> {
            fn from(val: PathOrId<'a>) -> Self {
                match val {
                    PathOrId::Path(cow) => match cow {
                        Cow::Owned(path) => Cow::Owned(path.to_string()),
                        Cow::Borrowed(path) => Cow::Borrowed(path.as_str()),
                    },
                    PathOrId::Id(id) => Cow::Borrowed(id),
                }
            }
        }

        let source = match package_metadata.source() {
            guppy::graph::PackageSource::Workspace(p) => {
                if !cache_workspace_packages {
                    return None;
                }
                let p = package_graph.workspace().root().join(p);
                PathOrId::Path(Cow::Owned(p))
            }
            guppy::graph::PackageSource::Path(p) => PathOrId::Path(Cow::Borrowed(p)),
            guppy::graph::PackageSource::External(e) => PathOrId::Id(e),
        };
        let crate_hash = if let PathOrId::Path(package_path) = &source {
            let package_path = if package_path.is_relative() {
                package_graph.workspace().root().join(package_path)
            } else {
                package_path.clone().into_owned()
            };
            // We need to compute the hash of the package's contents,
            // to invalidate the cache when the package changes.
            // This is only relevant for path dependencies.
            // We don't need to do this for external dependencies,
            // since they are assumed to be immutable.
            let hash = match checksum_crate(&package_path) {
                Ok(hash) => hash,
                Err(e) => {
                    log_error!(
                        *e,
                        "Failed to compute the hash of the package at {}. \
                            I won't cache its JSON documentation to avoid serving stale data.",
                        package_metadata.id().repr()
                    );
                    return None;
                }
            };
            Some(hash.to_string())
        } else {
            None
        };
        let feature_graph = package_graph.feature_graph();
        let feature_set = feature_graph
            .query_workspace(StandardFeatures::Default)
            .resolve();
        let features = feature_set
            .features_for(package_metadata.id())
            .expect("Failed to determine cargo features");
        let (default_feature_is_enabled, mut active_named_features) = match features {
            Some(f) => (f.has_base(), f.named_features().collect()),
            None => (false, vec![]),
        };
        active_named_features.sort();
        let cache_key = ThirdPartyCrateCacheKey {
            crate_name: package_metadata.name(),
            crate_source: source.into(),
            crate_version: package_metadata.version().to_string(),
            crate_hash,
            cargo_fingerprint,
            default_feature_is_enabled,
            // SQLite doesn't support arrays, so we have to serialize these two collections as strings.
            // This is well defined, since we sorted features and the order of options is well-defined.
            rustdoc_options: rustdoc_options().join(" "),
            active_named_features: active_named_features.join(" "),
        };
        Some(cache_key)
    }
}
