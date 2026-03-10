use std::borrow::Cow;
use std::collections::BTreeSet;
use std::sync::Arc;

use ahash::{HashMap, HashSet, HashSetExt};
use elsa::FrozenMap;
use guppy::PackageId;
use guppy::graph::PackageGraph;
use rustdoc_types::Item;
use tracing::Span;
use tracing_log_error::log_error;

use crate::TOOLCHAIN_CRATES;
use crate::cache::{CacheEntry, HydratedCacheEntry, RustdocCacheKey, RustdocGlobalFsCache};
use crate::compute::{CannotGetCrateData, ComputeProgress, compute_crate_docs};
use crate::indexing::CrateIndexer;
use crate::queries::Crate;
use rustdoc_ext::GlobalItemId;

use rayon::iter::IntoParallelRefIterator;

/// The main entrypoint for accessing the documentation of the crates
/// in a specific `PackageGraph`.
///
/// It takes care of:
/// - Computing and caching the JSON documentation for crates in the graph;
/// - Execute queries that span the documentation of multiple crates (e.g. following crate
///   re-exports or star re-exports).
///
/// # Performance
///
/// It takes a bit of engineering to build a reasonably-fast tool on `rustdoc-json`.
/// On its own, `rustdoc-json` is slow.
/// Furthermore, retrieval of cached entries is expensive too, given the size
/// of the JSON documentation for some crates.
///
/// `CrateCollection` uses a layered strategy to improve the performance of the
/// application using it.
///
/// ## Lazy Computation
///
/// To mitigate the performance impact, [`CrateCollection`] is _lazy_. It won't eagerly
/// compute the documentation for every single package in the graph.
/// Instead it'll populate entries whenever they are needed.
///
/// ## Bootstrapping
///
/// At the same time, computing entries one by one is significantly slower than having
/// `cargo` leveraging all available cores on the host machine.
/// To balance the two concerns, [`CrateCollection::bootstrap`] keeps track
/// of the crates which where needed the last time a collection was instantiated for the
/// workspace that's being analyzed.
///
/// ## Batching
///
/// Leverage [`CrateCollection::batch_compute_crates`] whenever you need to compute JSON
/// docs for multiple crates at once.
///
/// ## Parallelism
///
/// Internally, `CrateCollection` leverages `rayon` to offload CPU-intensive tasks to other
/// threads. E.g. deserialization of a raw JSON doc, chunks of the indexing pipeline, etc.
pub struct CrateCollection<I: CrateIndexer> {
    /// The graph underpinning this collection.
    ///
    /// Every single JSON document in this collection is attached
    /// to a specific [`PackageId`] in this graph.
    package_graph: PackageGraph,
    /// A mechanism to extract items out of the "raw" JSON documentation
    /// for a crate.
    /// In particular, it allows different consumers of this collection
    /// to extract project-specific annotations. Check out [`CrateIndexer`]
    /// for more information.
    indexer: I,
    /// A map to associate each [`PackageId`] to its (parsed) JSON documentation.
    package_id2krate: FrozenMap<PackageId, Box<Crate>>,
    /// Project-specific annotations extracted from items in each crate,
    /// stored as a side map to keep [`Crate`] itself project-agnostic.
    annotated_items: FrozenMap<PackageId, Box<I::Annotations>>,
    /// A SQLite cache, storing pre-computed JSON docs.
    ///
    /// It amortizes the cost of invoking `rustdoc-json` when the same workspace
    /// is analyzed a second time.
    disk_cache: RustdocGlobalFsCache<I::Annotations>,
    /// An opaque string that uniquely identifies the current project.
    project_fingerprint: String,
    /// This map keeps track of the packages that have been accessed while analyzing
    /// the underlying [`PackageGraph`].
    ///
    /// It is used to pre-compute/eagerly retrieve from the cache the docs for
    /// this project the next it's analyzed. See [`CrateCollection`] documentation for
    /// more information about performance optimization.
    ///
    /// # Implementation notes
    ///
    /// `elsa` doesn't expose a frozen BTreeSet yet, so we use a map with empty values
    /// to emulate it.
    access_log: FrozenMap<PackageId, Box<()>>,
    /// The name of the toolchain used to generate the JSON documentation of a crate.
    /// It is assumed to be a toolchain available via `rustup`.
    toolchain_name: String,
    /// Progress reporter for documenting crates.
    progress: Box<dyn ComputeProgress + Send + Sync>,
}

impl<I: CrateIndexer> std::fmt::Debug for CrateCollection<I> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CrateCollection")
            .field("package_graph", &self.package_graph)
            .finish_non_exhaustive()
    }
}

impl<I: CrateIndexer> CrateCollection<I> {
    /// Create a new [`CrateCollection`] with the given indexer and pre-constructed cache.
    pub fn new(
        indexer: I,
        toolchain_name: String,
        package_graph: PackageGraph,
        project_fingerprint: String,
        disk_cache: RustdocGlobalFsCache<I::Annotations>,
        progress: Box<dyn ComputeProgress + Send + Sync>,
    ) -> Self {
        Self {
            indexer,
            package_id2krate: FrozenMap::new(),
            annotated_items: FrozenMap::new(),
            package_graph,
            disk_cache,
            project_fingerprint,
            access_log: FrozenMap::new(),
            toolchain_name,
            progress,
        }
    }

    /// The graph underpinning this collection.
    ///
    /// Every single JSON document in this collection is attached
    /// to a specific [`PackageId`] in this graph.
    pub fn package_graph(&self) -> &PackageGraph {
        &self.package_graph
    }

    /// Bootstrap the crate collection by either fetching from the cache or computing
    /// on the fly the documentation of all the crates whose JSON docs are likely to be
    /// needed to process the application blueprint.
    ///
    /// We rely on:
    ///
    /// - Data from the previous run for this project, if available (the access log);
    /// - Heuristics (toolchain crates);
    /// - Data from the raw blueprint before any expensive processing has taken place (extra package ids).
    ///
    /// This method should only be called once.
    #[tracing::instrument(skip_all, level = "trace")]
    pub fn bootstrap<Iter>(&self, extra_package_ids: Iter) -> Result<(), anyhow::Error>
    where
        Iter: Iterator<Item = PackageId>,
    {
        let package_ids = self
            .disk_cache
            .get_access_log(&self.project_fingerprint)
            .unwrap_or_else(|e| {
                log_error!(
                    *e,
                    level: tracing::Level::WARN,
                    "Failed to retrieve the crate access log from the on-disk cache"
                );
                // This is an optimisation, therefore we should not
                // fail if the retrieval fails.
                BTreeSet::new()
            });
        let package_ids = package_ids
            .into_iter()
            .chain(extra_package_ids)
            // Some package ids may not longer be part of the package graph,
            // e.g. because the lockfile has been updated to use more recent versions
            // or they have been removed as dependencies from one or more of the Cargo.toml
            // manifests.
            .filter(|id| self.package_graph.metadata(id).is_ok())
            // We always need the documentation for the toolchain crates.
            .chain(TOOLCHAIN_CRATES.iter().map(|s| PackageId::new(*s)))
            .collect::<BTreeSet<_>>();
        self.compute_batch(package_ids.into_iter())
    }

    /// Process a [`HydratedCacheEntry`] into a `(Crate, Annotations)` pair,
    /// re-indexing if only the raw data was cached.
    fn process_cache_entry(
        &self,
        entry: HydratedCacheEntry<I::Annotations>,
        package_id: PackageId,
    ) -> (Crate, I::Annotations) {
        match entry {
            HydratedCacheEntry::Processed(processed) => processed.into_crate(),
            HydratedCacheEntry::Raw(crate_data) => {
                let result = self.indexer.index(crate_data, package_id);
                (result.krate, result.annotations)
            }
        }
    }

    /// Compute the documentation for multiple crates given their [`PackageId`]s.
    ///
    /// They won't be computed again if they are already in [`CrateCollection`]'s internal cache.
    #[tracing::instrument(skip_all, level = "trace")]
    pub fn compute_batch<Iter>(&self, package_ids: Iter) -> Result<(), anyhow::Error>
    where
        Iter: Iterator<Item = PackageId>,
    {
        let missing_ids = package_ids
            // First check if we already have the crate docs in the in-memory cache.
            .filter(|package_id| self.get_crate_by_package_id(package_id).is_none())
            .collect::<BTreeSet<_>>();

        // It can take a while to deserialize the JSON docs for a crate from the cache,
        // so we parallelize the operation.
        let package_graph = &self.package_graph;
        let cache = &self.disk_cache;
        let tracing_span = Span::current();
        let map_op = move |id: PackageId| {
            tracing_span.in_scope(|| {
                let cache_key = RustdocCacheKey::new(&id, package_graph);
                match cache.get(&cache_key, package_graph) {
                    Ok(None) => (id, None),
                    Ok(Some(entry)) => (id, Some(entry)),
                    Err(e) => {
                        log_error!(
                            *e,
                            level: tracing::Level::WARN,
                            package_id = id.repr(),
                            "Failed to retrieve the documentation from the on-disk cache",
                        );
                        (id, None)
                    }
                }
            })
        };

        let mut to_be_computed = vec![];

        use rayon::prelude::{IntoParallelIterator, ParallelIterator};
        for (package_id, cached) in missing_ids.into_par_iter().map(map_op).collect::<Vec<_>>() {
            if let Some(entry) = cached {
                let (krate, annotations) = self.process_cache_entry(entry, package_id.clone());
                self.annotated_items
                    .insert(package_id.clone(), Box::new(annotations));
                self.package_id2krate.insert(package_id, Box::new(krate));
                continue;
            }
            to_be_computed.push(package_id);
        }

        // The ones that are still missing need to be computed.
        let results = compute_crate_docs(
            &self.toolchain_name,
            &self.package_graph,
            to_be_computed.into_iter(),
            self.package_graph.workspace().root().as_std_path(),
            self.progress.as_ref(),
        )?;

        // We then have to perform two more expensive operations: indexing of all the items in each
        // crate and conversion of the "raw" JSON format into our optimised cache entry format.
        // We perform both in parallel, since they're CPU-intensive.
        //
        // First indexing:
        let indexer = &self.indexer;
        let package_graph = self.package_graph();
        let indexed_krates = results
            .into_par_iter()
            .map(move |(package_id, krate)| {
                let result = indexer.index_raw(krate, package_id.to_owned());
                (
                    package_id,
                    result.krate,
                    result.annotations,
                    result.can_cache_indexes,
                )
            })
            .collect::<Vec<_>>();
        // Then conversion to the desired cache format:
        let mut cache_entries: HashMap<_, _> = indexed_krates
            .par_iter()
            .filter_map(|(package_id, krate, annotations, cache_indexes)| {
                let data = if *cache_indexes {
                    CacheEntry::from_crate(krate, annotations)
                } else {
                    CacheEntry::from_crate_raw(krate)
                };
                let cache_key = RustdocCacheKey::new(package_id, package_graph);
                match data {
                    Ok(v) => Some((package_id, (cache_key, v))),
                    Err(e) => {
                        log_error!(
                            *e,
                            level: tracing::Level::WARN,
                            package_id = package_id.repr(),
                            "Failed to convert the computed JSON docs into the format used by the on-disk cache",
                        );
                        None
                    }
                }
            })
            .collect();

        let mut to_be_inserted = HashSet::with_capacity(indexed_krates.len());
        for (package_id, _, _, _) in &indexed_krates {
            let Some((cache_key, cache_data)) = cache_entries.remove(&package_id) else {
                continue;
            };
            if let Err(e) = self
                .disk_cache
                .insert(&cache_key, cache_data, package_graph)
            {
                log_error!(
                    *e,
                    level: tracing::Level::WARN,
                    package_id = package_id.repr(),
                    "Failed to store the computed JSON docs in the on-disk cache",
                );
            }
            // If we tried to insert into the in-memory cache directly, we'd get a borrow-checker
            // error since `cache_data` borrows from `krate`.
            // We keep track of what needs to be inserted and do it later once the on-disk
            // cache has been taken care of.
            to_be_inserted.insert(package_id.to_owned());
        }

        for (package_id, krate, annotations, _) in indexed_krates {
            if to_be_inserted.contains(&package_id) {
                self.annotated_items
                    .insert(package_id.clone(), Box::new(annotations));
                self.package_id2krate.insert(package_id, Box::new(krate));
            }
        }
        Ok(())
    }

    /// Compute the documentation for the crate associated with a specific [`PackageId`].
    ///
    /// It will be retrieved from [`CrateCollection`]'s internal cache if it was computed before.
    pub fn get_or_compute(&self, package_id: &PackageId) -> Result<&Crate, CannotGetCrateData> {
        self.access_log.insert(package_id.to_owned(), Box::new(()));

        // First check if we already have the crate docs in the in-memory cache.
        if let Some(krate) = self.get_crate_by_package_id(package_id) {
            return Ok(krate);
        }

        // If not, let's try to retrieve them from the on-disk cache.
        let cache_key = RustdocCacheKey::new(package_id, &self.package_graph);
        match self.disk_cache.get(&cache_key, &self.package_graph) {
            Ok(Some(entry)) => {
                let (krate, annotations) = self.process_cache_entry(entry, package_id.clone());
                self.annotated_items
                    .insert(package_id.to_owned(), Box::new(annotations));
                self.package_id2krate
                    .insert(package_id.to_owned(), Box::new(krate));
                return Ok(self.get_crate_by_package_id(package_id).unwrap());
            }
            Err(e) => {
                log_error!(*e, level: tracing::Level::WARN, package_id = package_id.repr(), "Failed to retrieve the documentation from the on-disk cache");
            }
            Ok(None) => {}
        }

        // If we don't have them in the on-disk cache, we need to compute them.
        let krate = compute_crate_docs(
            &self.toolchain_name,
            &self.package_graph,
            std::iter::once(package_id.to_owned()),
            self.package_graph.workspace().root().as_std_path(),
            self.progress.as_ref(),
        )
        .map_err(|e| CannotGetCrateData {
            package_spec: package_id.to_string(),
            source: Arc::new(e),
        })?
        .remove(package_id)
        .unwrap();

        let result = self.indexer.index_raw(krate, package_id.to_owned());

        // Store in the on-disk cache for next time.
        let cache_entry_data = if result.can_cache_indexes {
            CacheEntry::from_crate(&result.krate, &result.annotations)
        } else {
            CacheEntry::from_crate_raw(&result.krate)
        };
        match cache_entry_data {
            Ok(cache_entry) => {
                if let Err(e) = self
                    .disk_cache
                    .insert(&cache_key, cache_entry, &self.package_graph)
                {
                    log_error!(
                        *e,
                        level: tracing::Level::WARN,
                        package_id = package_id.repr(),
                        "Failed to store the computed JSON docs in the on-disk cache",
                    );
                }
            }
            Err(e) => {
                log_error!(
                    *e,
                    level: tracing::Level::WARN,
                    package_id = package_id.repr(),
                    "Failed to convert the computed JSON docs into the format used by the on-disk cache",
                );
            }
        }

        self.annotated_items
            .insert(package_id.to_owned(), Box::new(result.annotations));
        self.package_id2krate
            .insert(package_id.to_owned(), Box::new(result.krate));
        Ok(self.get_crate_by_package_id(package_id).unwrap())
    }

    /// Retrieve the documentation for the crate associated with [`PackageId`] from
    /// [`CrateCollection`]'s internal cache if it was computed before.
    ///
    /// It returns `None` if no documentation is found for the specified [`PackageId`].
    pub fn get_crate_by_package_id(&self, package_id: &PackageId) -> Option<&Crate> {
        self.package_id2krate.get(package_id)
    }

    /// Retrieve the annotations for a specific package, if available.
    pub fn get_annotated_items(&self, package_id: &PackageId) -> Option<&I::Annotations> {
        self.annotated_items.get(package_id)
    }

    /// Retrieve type information given its [`GlobalItemId`].
    ///
    /// It panics if no item is found for the specified [`GlobalItemId`].
    pub fn get_item_by_global_type_id(&self, type_id: &GlobalItemId) -> Cow<'_, Item> {
        let krate = self.get_or_compute(&type_id.package_id).unwrap();
        krate.get_item_by_local_type_id(&type_id.rustdoc_item_id)
    }

    /// Retrieve the canonical path for a struct, enum or function given its [`GlobalItemId`].
    ///
    /// It panics if no item is found for the specified [`GlobalItemId`].
    pub fn get_canonical_path_by_global_type_id(
        &self,
        type_id: &GlobalItemId,
    ) -> Result<&[String], anyhow::Error> {
        let krate = self.get_or_compute(&type_id.package_id).unwrap();
        krate.get_canonical_path(type_id)
    }

    /// Retrieve the canonical path and the [`GlobalItemId`] for a struct, enum or function given
    /// its **local** id.
    pub fn get_canonical_path_by_local_type_id(
        &self,
        used_by_package_id: &PackageId,
        item_id: &rustdoc_types::Id,
        // The item might come from a transitive dependency via a re-export
        // done by a direct dependency.
        // We don't have a bulletproof way of finding the re-exporter name, but we can
        // try to infer it (e.g. via the `name` property).
        re_exporter_crate_name: Option<&str>,
    ) -> Result<(GlobalItemId, &[String]), anyhow::Error> {
        let (definition_package_id, path) = {
            let used_by_krate = self.get_or_compute(used_by_package_id)?;
            let local_type_summary = used_by_krate.get_summary_by_local_type_id(item_id)?;
            (
                used_by_krate.compute_package_id_for_crate_id_with_hint(
                    local_type_summary.crate_id,
                    self,
                    // If the type was re-exported from another crate, the two names here should not match.
                    // The one coming from the summary is the name of the crate where the type was defined.
                    // The one coming from the `maybe_reexport_from` is the name of the crate where the type
                    // was re-exported from and used by the crate we are currently processing.
                    if local_type_summary.path.first().map(|s| s.as_str()) != re_exporter_crate_name
                    {
                        re_exporter_crate_name
                    } else {
                        None
                    },
                )?,
                local_type_summary.path.clone(),
            )
        };
        let definition_krate = self.get_or_compute(&definition_package_id)?;
        let type_id = definition_krate.get_item_id_by_path(&path, self)??;
        let canonical_path = self.get_canonical_path_by_global_type_id(&type_id)?;
        Ok((type_id.clone(), canonical_path))
    }
}

impl<I: CrateIndexer> Drop for CrateCollection<I> {
    fn drop(&mut self) {
        let access_log = std::mem::take(&mut self.access_log);
        let package_ids = access_log.into_map().into_keys().collect();
        if let Err(e) = self
            .disk_cache
            .persist_access_log(&package_ids, &self.project_fingerprint)
        {
            log_error!(
                *e,
                level: tracing::Level::WARN,
                "Failed to persist the crate access log to the on-disk cache",
            );
        }
    }
}
