use std::{borrow::Cow, collections::BTreeSet};

use ahash::{HashMap, HashMapExt};
use anyhow::Context;
use guppy::{
    graph::{feature::StandardFeatures, PackageGraph, PackageMetadata},
    PackageId,
};
use itertools::Itertools;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;
use tracing::instrument;

use crate::rustdoc::queries::{CrateData, CrateItemIndex, LazyCrateItemIndex};

use super::{rustdoc_options, toolchain};

/// A cache for storing and retrieving pre-computed JSON documentation generated by `rustdoc`.
///  
/// The cache is shared across all Pavex projects of the current user.
/// It is stored on disk, in the user home directory, using a SQLite database.
#[derive(Debug, Clone)]
pub(crate) struct RustdocGlobalFsCache {
    cargo_fingerprint: String,
    third_party_cache: ThirdPartyCrateCache,
    toolchain_cache: ToolchainCache,
    connection_pool: r2d2::Pool<SqliteConnectionManager>,
}

pub(crate) enum RustdocCacheKey<'a> {
    ThirdPartyCrate(PackageMetadata<'a>),
    ToolchainCrate(&'a str),
}

impl<'a> RustdocCacheKey<'a> {
    pub fn new(package_id: &'a PackageId, package_graph: &'a PackageGraph) -> RustdocCacheKey<'a> {
        if crate::rustdoc::TOOLCHAIN_CRATES.contains(&package_id.repr()) {
            RustdocCacheKey::ToolchainCrate(package_id.repr())
        } else {
            RustdocCacheKey::ThirdPartyCrate(package_graph.metadata(package_id).unwrap())
        }
    }
}

impl RustdocGlobalFsCache {
    /// Initialize a new instance of the cache.
    #[tracing::instrument(name = "Initialize on-disk rustdoc cache", skip_all)]
    pub(crate) fn new() -> Result<Self, anyhow::Error> {
        let cargo_fingerprint = cargo_fingerprint()?;
        let pool = Self::setup_database()?;

        let connection = pool.get()?;
        let third_party_cache = ThirdPartyCrateCache::new(&connection)?;
        let toolchain_cache = ToolchainCache::new(&connection)?;
        Ok(Self {
            cargo_fingerprint,
            connection_pool: pool,
            third_party_cache,
            toolchain_cache,
        })
    }

    /// Retrieve the cached documentation for a given package, if available.
    pub(crate) fn get(
        &self,
        cache_key: &RustdocCacheKey,
    ) -> Result<Option<crate::rustdoc::Crate>, anyhow::Error> {
        let connection = self.connection_pool.get()?;
        match cache_key {
            RustdocCacheKey::ThirdPartyCrate(metadata) => {
                self.third_party_cache
                    .get(metadata, &self.cargo_fingerprint, &connection)
            }
            RustdocCacheKey::ToolchainCrate(name) => {
                self.toolchain_cache
                    .get(name, &self.cargo_fingerprint, &connection)
            }
        }
    }

    /// Store the JSON documentation generated by `rustdoc` in the cache.
    pub(crate) fn insert(
        &self,
        cache_key: &RustdocCacheKey,
        krate: &crate::rustdoc::Crate,
    ) -> Result<(), anyhow::Error> {
        let connection = self.connection_pool.get()?;
        match cache_key {
            RustdocCacheKey::ThirdPartyCrate(metadata) => {
                self.third_party_cache
                    .insert(metadata, krate, &self.cargo_fingerprint, &connection)
            }
            RustdocCacheKey::ToolchainCrate(name) => {
                self.toolchain_cache
                    .insert(name, krate, &self.cargo_fingerprint, &connection)
            }
        }
    }

    #[tracing::instrument(skip_all, level = "trace")]
    /// Persist the list of package IDs that were accessed during the processing of the
    /// application blueprint for this project.
    pub(crate) fn persist_access_log(
        &self,
        package_ids: &BTreeSet<PackageId>,
        project_fingerprint: &str,
    ) -> Result<(), anyhow::Error> {
        let connection = self.connection_pool.get()?;

        let mut stmt = connection.prepare_cached(
            "INSERT INTO project2package_id_access_log (
                project_fingerprint,
                package_ids
            ) VALUES (?, ?)
            ON CONFLICT(project_fingerprint) DO UPDATE SET package_ids=excluded.package_ids;
            ",
        )?;
        stmt.execute(params![
            project_fingerprint,
            bincode::serialize(&package_ids.iter().map(|s| s.repr()).collect_vec())?
        ])?;

        Ok(())
    }

    #[tracing::instrument(skip_all, level = "trace")]
    /// Retrieve the list of package IDs that were accessed during the last time we processed the  
    /// application blueprint for this project.
    ///
    /// Returns an empty set if no access log is found for the given project fingerprint.
    pub(crate) fn get_access_log(
        &self,
        project_fingerprint: &str,
    ) -> Result<BTreeSet<PackageId>, anyhow::Error> {
        let connection = self.connection_pool.get()?;

        let mut stmt = connection.prepare_cached(
            "SELECT package_ids FROM project2package_id_access_log WHERE project_fingerprint = ?",
        )?;
        let mut rows = stmt.query(params![project_fingerprint])?;
        let Some(row) = rows.next()? else {
            return Ok(BTreeSet::new());
        };

        let package_ids: Vec<&str> = bincode::deserialize(row.get_ref_unwrap(0).as_bytes()?)?;
        Ok(package_ids.into_iter().map(PackageId::new).collect())
    }

    /// Initialize the database, creating the file and the relevant tables if they don't exist yet.
    fn setup_database() -> Result<r2d2::Pool<SqliteConnectionManager>, anyhow::Error> {
        let pavex_fingerprint =
            concat!(env!("CARGO_PKG_VERSION"), '-', env!("VERGEN_GIT_DESCRIBE"));
        let cache_dir = xdg_home::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Failed to get the user's home directory"))?
            .join(".pavex/rustdoc/cache");
        fs_err::create_dir_all(&cache_dir).with_context(|| {
            format!(
                "Failed to create the cache directory at {}",
                cache_dir.to_string_lossy()
            )
        })?;

        // For the sake of simplicity, we use a different SQLite database for each version of Pavex.
        // This ensures that we don't have to worry about schema migrations.
        // The cost we pay: the user will have to re-generate the documentation for all their crates
        // when they upgrade Pavex.
        // We can improve this in the future, if needed.
        let cache_path = cache_dir.join(format!("{}.db", pavex_fingerprint));

        let manager = SqliteConnectionManager::file(cache_dir.join(cache_path));
        let pool = r2d2::Pool::builder()
            .max_size(num_cpus::get() as u32)
            .build(manager)
            .context("Failed to open/create a SQLite database to store the contents of pavex's rustdoc cache")?;

        let connection = pool.get()?;
        connection.execute(
            "CREATE TABLE IF NOT EXISTS project2package_id_access_log (
                project_fingerprint TEXT NOT NULL,
                package_ids BLOB NOT NULL,
                PRIMARY KEY (project_fingerprint)
            )",
            [],
        )?;

        Ok(pool)
    }
}

#[derive(Debug, Clone)]
#[non_exhaustive]
struct ToolchainCache {}

impl ToolchainCache {
    fn new(connection: &rusqlite::Connection) -> Result<Self, anyhow::Error> {
        Self::setup_table(connection)?;
        Ok(Self {})
    }

    /// Retrieve the cached documentation for a given toolchain crate, if available.
    #[instrument(name = "Retrieve cached toolchain docs from disk", 
        skip_all,
        level=tracing::Level::DEBUG,
        fields(crate.name = %name)
    )]
    fn get(
        &self,
        name: &str,
        cargo_fingerprint: &str,
        connection: &rusqlite::Connection,
    ) -> Result<Option<crate::rustdoc::Crate>, anyhow::Error> {
        // Retrieve from rustdoc's output from cache, if available.
        let mut stmt = connection.prepare_cached(
            "SELECT 
                root_item_id,
                external_crates,
                paths,
                format_version,
                items,
                item_id2delimiters,
                id2public_import_paths,
                id2private_import_paths,
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

        let root_item_id = row.get_ref_unwrap(0).as_str()?;
        let external_crates = row.get_ref_unwrap(1).as_bytes()?;
        let paths = row.get_ref_unwrap(2).as_bytes()?;
        let format_version = row.get_ref_unwrap(3).as_i64()?;

        let span = tracing::trace_span!("Copy items bytes buffer");
        let guard = span.enter();
        let items: Vec<u8> = row.get_unwrap(4);
        drop(guard);

        let item_id2delimiters = row.get_ref_unwrap(5).as_bytes()?;
        let id2public_import_paths = row.get_ref_unwrap(6).as_bytes()?;
        let id2private_import_paths = row.get_ref_unwrap(7).as_bytes()?;
        let import_path2id = row.get_ref_unwrap(8).as_bytes()?;
        let re_exports = row.get_ref_unwrap(9).as_bytes()?;

        let krate = CachedData {
            root_item_id,
            external_crates: Cow::Borrowed(external_crates),
            paths: Cow::Borrowed(paths),
            format_version,
            items: Cow::Owned(items),
            item_id2delimiters: Cow::Borrowed(item_id2delimiters),
            id2public_import_paths: Cow::Borrowed(id2public_import_paths),
            id2private_import_paths: Cow::Borrowed(id2private_import_paths),
            import_path2id: Cow::Borrowed(import_path2id),
            re_exports: Cow::Borrowed(re_exports),
        }
        .hydrate(PackageId::new(name))?;

        Ok(Some(krate))
    }

    /// Store the JSON documentation for a toolchain crate in the cache.
    #[instrument(name = "Cache rustdoc output on disk", skip_all, level=tracing::Level::DEBUG, fields(crate.name = name))]
    fn insert(
        &self,
        name: &str,
        krate: &crate::rustdoc::Crate,
        cargo_fingerprint: &str,
        connection: &rusqlite::Connection,
    ) -> Result<(), anyhow::Error> {
        let cached_data = CachedData::new(krate).context("Failed to serialize docs")?;
        let mut stmt = connection.prepare_cached(
            "INSERT INTO rustdoc_toolchain_crates_cache (
                name,
                cargo_fingerprint,
                root_item_id,
                external_crates,
                paths,
                format_version,
                items,
                item_id2delimiters,
                id2public_import_paths,
                id2private_import_paths,
                import_path2id,
                re_exports
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )?;
        stmt.execute(params![
            name,
            cargo_fingerprint,
            cached_data.root_item_id,
            cached_data.external_crates,
            cached_data.paths,
            cached_data.format_version,
            cached_data.items,
            cached_data.item_id2delimiters,
            cached_data.id2public_import_paths,
            cached_data.id2private_import_paths,
            cached_data.import_path2id,
            cached_data.re_exports
        ])?;
        Ok(())
    }

    fn setup_table(connection: &rusqlite::Connection) -> Result<(), anyhow::Error> {
        connection.execute(
            "CREATE TABLE IF NOT EXISTS rustdoc_toolchain_crates_cache (
                name TEXT NOT NULL,
                cargo_fingerprint TEXT NOT NULL,
                root_item_id TEXT NOT NULL,
                external_crates BLOB NOT NULL,
                paths BLOB NOT NULL,
                format_version INTEGER NOT NULL,
                items BLOB NOT NULL,
                item_id2delimiters BLOB NOT NULL,
                id2public_import_paths BLOB NOT NULL,
                id2private_import_paths BLOB NOT NULL,
                import_path2id BLOB NOT NULL,
                re_exports BLOB NOT NULL,
                PRIMARY KEY (name, cargo_fingerprint)
            )",
            [],
        )?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
#[non_exhaustive]
struct ThirdPartyCrateCache {}

impl ThirdPartyCrateCache {
    fn new(connection: &rusqlite::Connection) -> Result<Self, anyhow::Error> {
        Self::setup_table(connection)?;
        Ok(Self {})
    }

    /// Retrieve the cached documentation for a given package, if available.
    #[instrument(name = "Retrieve cached toolchain docs from disk", 
        skip_all,
        level=tracing::Level::DEBUG,
        fields(crate.name = %package_metadata.name())
    )]
    fn get(
        &self,
        package_metadata: &PackageMetadata,
        cargo_fingerprint: &str,
        connection: &rusqlite::Connection,
    ) -> Result<Option<crate::rustdoc::Crate>, anyhow::Error> {
        let Some(cache_key) = ThirdPartyCrateCacheKey::build(package_metadata, cargo_fingerprint)
        else {
            return Ok(None);
        };
        // Retrieve from rustdoc's output from cache, if available.
        let mut stmt = connection.prepare_cached(
            "SELECT 
                    root_item_id,
                    external_crates,
                    paths,
                    format_version,
                    items,
                    item_id2delimiters,
                    id2public_import_paths,
                    id2private_import_paths,
                    import_path2id, 
                    re_exports
                FROM rustdoc_3d_party_crates_cache 
                WHERE crate_name = ? AND 
                    crate_source = ? AND 
                    crate_version = ? AND 
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
            cache_key.cargo_fingerprint,
            cache_key.rustdoc_options,
            cache_key.default_feature_is_enabled,
            cache_key.active_named_features
        ])?;
        let Some(row) = rows.next()? else {
            return Ok(None);
        };
        drop(guard);

        let root_item_id = row.get_ref_unwrap(0).as_str()?;
        let external_crates = row.get_ref_unwrap(1).as_bytes()?;
        let paths = row.get_ref_unwrap(2).as_bytes()?;
        let format_version = row.get_ref_unwrap(3).as_i64()?;

        let span = tracing::trace_span!("Copy items bytes buffer");
        let guard = span.enter();
        let items: Vec<u8> = row.get_unwrap(4);
        drop(guard);

        let item_id2delimiters = row.get_ref_unwrap(5).as_bytes()?;
        let id2public_import_paths = row.get_ref_unwrap(6).as_bytes()?;
        let id2private_import_paths = row.get_ref_unwrap(7).as_bytes()?;
        let import_path2id = row.get_ref_unwrap(8).as_bytes()?;
        let re_exports = row.get_ref_unwrap(9).as_bytes()?;

        let krate = CachedData {
            root_item_id,
            external_crates: Cow::Borrowed(external_crates),
            paths: Cow::Borrowed(paths),
            format_version,
            items: Cow::Owned(items),
            item_id2delimiters: Cow::Borrowed(item_id2delimiters),
            id2public_import_paths: Cow::Borrowed(id2public_import_paths),
            id2private_import_paths: Cow::Borrowed(id2private_import_paths),
            import_path2id: Cow::Borrowed(import_path2id),
            re_exports: Cow::Borrowed(re_exports),
        }
        .hydrate(package_metadata.id().to_owned())
        .context("Failed to re-hydrate the stored docs")?;

        Ok(Some(krate))
    }

    /// Store the JSON documentation generated by `rustdoc` in the cache.
    #[instrument(name = "Cache rustdoc output on disk", skip_all, level=tracing::Level::DEBUG, fields(crate.name = %package_metadata.name()))]
    fn insert(
        &self,
        package_metadata: &PackageMetadata,
        krate: &crate::rustdoc::Crate,
        cargo_fingerprint: &str,
        connection: &rusqlite::Connection,
    ) -> Result<(), anyhow::Error> {
        let Some(cache_key) = ThirdPartyCrateCacheKey::build(package_metadata, cargo_fingerprint)
        else {
            return Ok(());
        };
        let cached_data = CachedData::new(krate).context("Failed to serialize docs")?;
        let mut stmt = connection.prepare_cached(
            "INSERT INTO rustdoc_3d_party_crates_cache (
                crate_name,
                crate_source,
                crate_version,
                cargo_fingerprint,
                rustdoc_options,
                default_feature_is_enabled,
                active_named_features,
                root_item_id,
                external_crates,
                paths,
                format_version,
                items,
                item_id2delimiters,
                id2public_import_paths,
                id2private_import_paths,
                import_path2id,
                re_exports
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )?;
        stmt.execute(params![
            cache_key.crate_name,
            cache_key.crate_source,
            cache_key.crate_version,
            cache_key.cargo_fingerprint,
            cache_key.rustdoc_options,
            cache_key.default_feature_is_enabled,
            cache_key.active_named_features,
            cached_data.root_item_id,
            cached_data.external_crates,
            cached_data.paths,
            cached_data.format_version,
            cached_data.items,
            cached_data.item_id2delimiters,
            cached_data.id2public_import_paths,
            cached_data.id2private_import_paths,
            cached_data.import_path2id,
            cached_data.re_exports
        ])?;
        Ok(())
    }

    fn setup_table(connection: &rusqlite::Connection) -> Result<(), anyhow::Error> {
        connection.execute(
            "CREATE TABLE IF NOT EXISTS rustdoc_3d_party_crates_cache (
                crate_name TEXT NOT NULL,
                crate_source TEXT NOT NULL,
                crate_version TEXT NOT NULL,
                cargo_fingerprint TEXT NOT NULL,
                rustdoc_options TEXT NOT NULL,
                default_feature_is_enabled INTEGER NOT NULL,
                active_named_features TEXT NOT NULL,
                root_item_id TEXT NOT NULL,
                external_crates BLOB NOT NULL,
                paths BLOB NOT NULL,
                format_version INTEGER NOT NULL,
                items BLOB NOT NULL,
                item_id2delimiters BLOB NOT NULL,
                id2public_import_paths BLOB NOT NULL,
                id2private_import_paths BLOB NOT NULL,
                import_path2id BLOB NOT NULL,
                re_exports BLOB NOT NULL,
                PRIMARY KEY (crate_name, crate_source, crate_version, cargo_fingerprint, rustdoc_options, default_feature_is_enabled, active_named_features)
            )",
            []
        )?;
        Ok(())
    }
}

#[derive(Debug)]
/// The serialized form of a crate's documentation, as stored in the cache.
pub(super) struct CachedData<'a> {
    root_item_id: &'a str,
    external_crates: Cow<'a, [u8]>,
    paths: Cow<'a, [u8]>,
    format_version: i64,
    items: Cow<'a, [u8]>,
    item_id2delimiters: Cow<'a, [u8]>,
    id2public_import_paths: Cow<'a, [u8]>,
    id2private_import_paths: Cow<'a, [u8]>,
    import_path2id: Cow<'a, [u8]>,
    re_exports: Cow<'a, [u8]>,
}

impl<'a> CachedData<'a> {
    pub(super) fn new(krate: &'a crate::rustdoc::Crate) -> Result<CachedData<'a>, anyhow::Error> {
        let crate_data = &krate.core.krate;
        let CrateItemIndex::Eager(index) = &crate_data.index else {
            anyhow::bail!(
                "The crate's item index is not deserialized. Are we trying to cache \
                the same crate twice? This is a bug."
            );
        };
        let mut items = Vec::new();
        let mut item_id2delimiters = HashMap::new();
        for (item_id, item) in &index.index {
            let start = items.len();
            serde_json::to_writer(&mut items, item)?;
            let end = items.len();
            item_id2delimiters.insert(item_id.0.as_str(), (start, end));
        }

        let id2public_import_paths = bincode::serialize(&krate.id2public_import_paths)?;
        let id2private_import_paths = bincode::serialize(&krate.id2private_import_paths)?;
        let import_path2id = bincode::serialize(&krate.import_path2id)?;
        let re_exports = bincode::serialize(&krate.re_exports)?;
        let external_crates = bincode::serialize(&crate_data.external_crates)?;
        let paths = bincode::serialize(&crate_data.paths)?;

        Ok(CachedData {
            root_item_id: crate_data.root_item_id.0.as_str(),
            external_crates: Cow::Owned(external_crates),
            paths: Cow::Owned(paths),
            format_version: crate_data.format_version as i64,
            items: Cow::Owned(items),
            item_id2delimiters: Cow::Owned(bincode::serialize(&item_id2delimiters)?),
            id2public_import_paths: Cow::Owned(id2public_import_paths),
            id2private_import_paths: Cow::Owned(id2private_import_paths),
            import_path2id: Cow::Owned(import_path2id),
            re_exports: Cow::Owned(re_exports),
        })
    }

    /// Re-hydrate the documentation retrieved from the cache.
    ///
    /// We hydrate all mappings eagerly, but we avoid re-hydrating the item index eagerly,
    /// since it can be quite large and deserialization can be slow for large crates.
    pub(super) fn hydrate(
        self,
        package_id: PackageId,
    ) -> Result<crate::rustdoc::Crate, anyhow::Error> {
        let span = tracing::trace_span!("Deserialize delimiters");
        let _guard = span.enter();
        let item_id2delimiters: HashMap<rustdoc_types::Id, (usize, usize)> =
            bincode::deserialize(&self.item_id2delimiters)
                .context("Failed to deserialize item_id2delimiters")?;
        drop(_guard);

        let span = tracing::trace_span!("Deserialize paths");
        let _guard = span.enter();
        let paths = bincode::deserialize(&self.paths)?;
        drop(_guard);

        let crate_data = CrateData {
            root_item_id: rustdoc_types::Id(self.root_item_id.to_owned()),
            external_crates: bincode::deserialize(&self.external_crates)?,
            paths,
            format_version: self.format_version.try_into()?,
            index: CrateItemIndex::Lazy(LazyCrateItemIndex {
                items: self.items.into_owned(),
                item_id2delimiters,
            }),
        };
        let core = crate::rustdoc::queries::CrateCore {
            package_id,
            krate: crate_data,
        };

        let span = tracing::trace_span!("Deserialize import_path2id");
        let _guard = span.enter();
        let import_path2id: HashMap<Vec<String>, rustdoc_types::Id> =
            bincode::deserialize(&self.import_path2id)
                .context("Failed to deserialize import_path2id")?;
        drop(_guard);

        let re_exports =
            bincode::deserialize(&self.re_exports).context("Failed to deserialize re-exports")?;

        let id2public_import_paths = bincode::deserialize(&self.id2public_import_paths)
            .context("Failed to deserialize id2public_import_paths")?;
        let id2private_import_paths = bincode::deserialize(&self.id2private_import_paths)
            .context("Failed to deserialize id2private_import_paths")?;

        let krate = crate::rustdoc::Crate {
            core,
            import_path2id,
            re_exports,
            id2private_import_paths,
            id2public_import_paths,
        };
        Ok(krate)
    }
}

/// The key used to store and retrieve a crate's documentation from the cache.
///
/// It tries to capture all the information that can influence the output of the
/// relevant `rustdoc` command.
#[derive(Debug)]
pub(super) struct ThirdPartyCrateCacheKey<'a> {
    pub crate_name: &'a str,
    pub crate_source: &'a str,
    pub crate_version: String,
    pub cargo_fingerprint: &'a str,
    pub rustdoc_options: String,
    pub default_feature_is_enabled: bool,
    pub active_named_features: String,
}

impl<'a> ThirdPartyCrateCacheKey<'a> {
    /// Compute the cache key for a given package.
    pub(super) fn build(
        package_metadata: &'a PackageMetadata<'a>,
        cargo_fingerprint: &'a str,
    ) -> Option<ThirdPartyCrateCacheKey<'a>> {
        // We don't want to cache the docs for workspace crates and path dependencies.
        let Some(source) = package_metadata.source().external_source() else {
            return None;
        };
        let features = package_metadata
            .to_feature_set(StandardFeatures::Default)
            .features_for(package_metadata.id())
            .unwrap();
        let (default_feature_is_enabled, mut active_named_features) = match features {
            Some(f) => (f.has_base(), f.named_features().collect()),
            None => (false, vec![]),
        };
        active_named_features.sort();
        let cache_key = ThirdPartyCrateCacheKey {
            crate_name: package_metadata.name(),
            crate_source: source,
            crate_version: package_metadata.version().to_string(),
            cargo_fingerprint,
            default_feature_is_enabled,
            // SQLite doesn't support arrays, so we have to serialize these two collections as strings.
            // This is well defined, since the order is well-defined.
            rustdoc_options: rustdoc_options().join(" "),
            active_named_features: active_named_features.join(" "),
        };
        Some(cache_key)
    }
}

/// Return the output of `cargo --verbose --version` for the nightly toolchain,
/// which can be used to fingerprint the toolchain used by Pavex.
pub fn cargo_fingerprint() -> Result<String, anyhow::Error> {
    let err_msg = || {
        "Failed to run `cargo --verbose --version` on `nightly`.\n
        Is the `nightly` toolchain installed?\n
        If not, invoke `rustup toolchain install nightly` to fix it."
    };
    let nightly_cargo_path = toolchain::get_nightly_cargo_via_rustup()?;
    let mut cmd = std::process::Command::new(nightly_cargo_path);
    cmd.arg("--verbose").arg("--version");
    let output = cmd.output().with_context(err_msg)?;
    if !output.status.success() {
        anyhow::bail!(err_msg());
    }
    let output = String::from_utf8(output.stdout).with_context(|| {
        "An invocation of `cargo --verbose --version` for the nightly toolchain returned non-UTF8 data as output."
    })?;
    Ok(output)
}
