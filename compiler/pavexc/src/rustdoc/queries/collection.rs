use std::borrow::Cow;
use std::collections::BTreeSet;
use std::sync::Arc;

use ahash::{HashMap, HashSet, HashSetExt};
use elsa::FrozenMap;
use guppy::PackageId;
use guppy::graph::PackageGraph;
use rustdoc_types::{Item, ItemEnum};
use tracing::Span;
use tracing_log_error::log_error;

use crate::compiler::resolvers::{GenericBindings, resolve_type};
use crate::language::{
    FQGenericArgument, FQPathType, UnknownCrate, krate2package_id, resolve_fq_path_type,
};
use crate::rustdoc::CannotGetCrateData;
use crate::rustdoc::{ALLOC_PACKAGE_ID, CORE_PACKAGE_ID, STD_PACKAGE_ID};
use rustdoc_ext::RustdocKindExt;
use rustdoc_processor::cache::{CacheEntry, HydratedCacheEntry, RustdocGlobalFsCache};
use rustdoc_processor::compute::compute_crate_docs;
use rustdoc_processor::indexing::CrateIndexer;

use super::super::AnnotatedItem;
use super::super::annotations::AnnotationCoordinates;
use super::super::compute::{CacheEntryExt, RustdocCacheKey};
use super::super::indexer::PavexIndexer;
use super::super::progress_reporter::ShellProgress;
use rustdoc_processor::queries::{Crate, CrateRegistry};
use rustdoc_processor::{GlobalItemId, UnknownItemPath};

use rayon::iter::IntoParallelRefIterator;

/// The main entrypoint for accessing the documentation of the crates
/// in a specific `PackageGraph`.
///
/// It takes care of:
/// - Computing and caching the JSON documentation for crates in the graph;
/// - Execute queries that span the documentation of multiple crates (e.g. following crate
///   re-exports or star re-exports).
pub struct CrateCollection<I: CrateIndexer = PavexIndexer> {
    indexer: I,
    package_id2krate: FrozenMap<PackageId, Box<Crate>>,
    /// Annotations extracted from each crate, stored as a side map
    /// to keep `Crate` itself annotation-agnostic.
    annotated_items: FrozenMap<PackageId, Box<I::Annotations>>,
    pub(super) package_graph: PackageGraph,
    disk_cache: RustdocGlobalFsCache<I::Annotations>,
    /// An opaque string that uniquely identifies the current project (i.e. the current
    /// blueprint and the crate we are generating from it).
    project_fingerprint: String,
    /// This map keeps track of the packages that are accessed while processing the
    /// blueprint of an application.
    ///
    /// This is then used to pre-compute/eagerly retrieve from the cache the docs for
    /// this crate the next time we process a blueprint for the same project.
    ///
    /// # Implementation notes
    /// `elsa` doesn't expose a frozen BTreeSet yet, so we use a map with empty values
    /// to emulate it.
    access_log: FrozenMap<PackageId, Box<()>>,
    /// The name of the toolchain used to generate the JSON documentation of a crate.
    /// It is assumed to be a toolchain available via `rustup`.
    toolchain_name: String,
}

impl<I: CrateIndexer> std::fmt::Debug for CrateCollection<I> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CrateCollection")
            .field("package_graph", &self.package_graph)
            .finish_non_exhaustive()
    }
}

impl<I: CrateIndexer> CrateCollection<I> {
    /// Create a new `CrateCollection` with the given indexer and pre-constructed cache.
    pub fn new(
        indexer: I,
        toolchain_name: String,
        package_graph: PackageGraph,
        project_fingerprint: String,
        disk_cache: RustdocGlobalFsCache<I::Annotations>,
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
        }
    }

    pub fn package_graph(&self) -> &PackageGraph {
        &self.package_graph
    }

    /// Bootstrap the crate collection by either fetching from the cache or computing
    /// on the fly the documentation of all the crates whose JSON docs are likely to be
    /// needed to process the application blueprint.
    ///
    /// We rely on:
    ///
    /// - Data from the previous run of Pavex for this project, if available (the access log);
    /// - Heuristics (toolchain crates);
    /// - Data from the raw blueprint before any expensive processing has taken place (extra package ids).
    ///
    /// This method should only be called once.
    #[tracing::instrument(skip_all, level = "trace")]
    pub fn bootstrap_collection<Iter>(&self, extra_package_ids: Iter) -> Result<(), anyhow::Error>
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
            .chain([
                CORE_PACKAGE_ID.to_owned(),
                ALLOC_PACKAGE_ID.to_owned(),
                STD_PACKAGE_ID.to_owned(),
            ])
            .collect::<BTreeSet<_>>();
        self.batch_compute_crates(package_ids.into_iter())
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
    pub fn batch_compute_crates<Iter>(&self, package_ids: Iter) -> Result<(), anyhow::Error>
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
                let (krate, annotations) =
                    self.process_cache_entry(entry, package_id.clone());
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
            &ShellProgress,
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
                (package_id, result.krate, result.annotations, result.can_cache_indexes)
            })
            .collect::<Vec<_>>();
        // Then conversion to the desired cache format:
        let mut cache_entries: HashMap<_, _> = indexed_krates
            .par_iter()
            .filter_map(|(package_id, krate, annotations, cache_indexes)| {
                let data = if *cache_indexes {
                    <CacheEntry as CacheEntryExt>::from_crate(krate, annotations)
                } else {
                    <CacheEntry as CacheEntryExt>::from_crate_raw(krate)
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
    pub fn get_or_compute_crate_by_package_id(
        &self,
        package_id: &PackageId,
    ) -> Result<&Crate, CannotGetCrateData> {
        self.access_log.insert(package_id.to_owned(), Box::new(()));

        // First check if we already have the crate docs in the in-memory cache.
        if let Some(krate) = self.get_crate_by_package_id(package_id) {
            return Ok(krate);
        }

        // If not, let's try to retrieve them from the on-disk cache.
        let cache_key = RustdocCacheKey::new(package_id, &self.package_graph);
        match self.disk_cache.get(&cache_key, &self.package_graph) {
            Ok(Some(entry)) => {
                let (krate, annotations) =
                    self.process_cache_entry(entry, package_id.clone());
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
            &ShellProgress,
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
            <CacheEntry as CacheEntryExt>::from_crate(&result.krate, &result.annotations)
        } else {
            <CacheEntry as CacheEntryExt>::from_crate_raw(&result.krate)
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
}

// Pavex-specific methods that only apply when using PavexIndexer.
impl CrateCollection<PavexIndexer> {
    /// Convenience constructor for Pavex: creates a `PavexIndexer` and the
    /// pre-configured on-disk cache, then returns a ready-to-use collection.
    pub fn new_pavex(
        toolchain_name: String,
        package_graph: PackageGraph,
        project_fingerprint: String,
        cache_workspace_package_docs: bool,
        diagnostic_sink: crate::diagnostic::DiagnosticSink,
    ) -> Result<Self, anyhow::Error> {
        let disk_cache = super::super::compute::pavex_rustdoc_cache(
            &toolchain_name,
            cache_workspace_package_docs,
            &package_graph,
        )?;
        let indexer = PavexIndexer::new(diagnostic_sink);
        Ok(Self::new(
            indexer,
            toolchain_name,
            package_graph,
            project_fingerprint,
            disk_cache,
        ))
    }

    /// Retrieve the annotation associated with the given item, if any.
    pub fn annotation(&self, item_id: &GlobalItemId) -> Option<&AnnotatedItem> {
        self.annotated_items
            .get(&item_id.package_id)?
            .get_by_item_id(item_id.rustdoc_item_id)
    }

    /// Retrieve the annotation that these coordinates point to, if any.
    #[allow(clippy::type_complexity)]
    pub fn annotation_for_coordinates(
        &self,
        c: &AnnotationCoordinates,
    ) -> Result<Result<Option<(&Crate, &AnnotatedItem)>, UnknownCrate>, CannotGetCrateData> {
        let package_id = match krate2package_id(
            &c.created_at.package_name,
            &c.created_at.package_version,
            &self.package_graph,
        ) {
            Ok(p) => p,
            Err(e) => return Ok(Err(e)),
        };
        let krate = self.get_or_compute_crate_by_package_id(&package_id)?;
        let annotations = self.annotated_items.get(&package_id);
        Ok(Ok(annotations
            .and_then(|a| a.get_by_annotation_id(&c.id))
            .map(|item| (krate, item))))
    }

    pub fn get_type_by_resolved_path(
        &self,
        mut resolved_path: crate::language::FQPath,
    ) -> Result<
        Result<(crate::language::FQPath, ResolvedItem<'_>), GetItemByResolvedPathError>,
        CannotGetCrateData,
    > {
        let mut path_without_generics = resolved_path
            .segments
            .iter()
            .map(|p| p.ident.clone())
            .collect::<Vec<_>>();
        let krate = self.get_or_compute_crate_by_package_id(&resolved_path.package_id)?;
        // The path may come from a crate that depends on the one we are re-examining
        // but with a rename in its `Cargo.toml`. We normalize the path to the original crate name
        // in order to get a match in the index.
        path_without_generics[0] = krate.crate_name();

        let Ok(mut type_id) = krate.get_item_id_by_path(&path_without_generics, self)? else {
            return Ok(Err(UnknownItemPath {
                path: path_without_generics,
            }
            .into()));
        };

        let mut item = self.get_item_by_global_type_id(&type_id);

        if !matches!(
            item.inner,
            ItemEnum::Struct(_)
                | ItemEnum::Enum(_)
                | ItemEnum::TypeAlias(_)
                | ItemEnum::Trait(_)
                | ItemEnum::Primitive(_)
        ) {
            return Ok(Err(GetItemByResolvedPathError::UnsupportedItemKind(
                UnsupportedItemKind {
                    path: path_without_generics,
                    kind: item.inner.kind().into(),
                },
            )));
        }

        // We eagerly check if the item is an alias, and if so we follow it
        // to the original type.
        // This process might take multiple iterations, since the alias might point to another
        // alias, recursively.
        let mut krate = self.get_or_compute_crate_by_package_id(&type_id.package_id)?;
        loop {
            let ItemEnum::TypeAlias(type_alias) = &item.inner else {
                break;
            };
            let rustdoc_types::Type::ResolvedPath(aliased_path) = &type_alias.type_ else {
                break;
            };

            // The aliased type might be a re-export of a foreign type,
            // therefore we go through the summary here rather than
            // going straight for a local id lookup.
            let aliased_summary = krate
                .get_summary_by_local_type_id(&aliased_path.id)
                .unwrap();
            let aliased_package_id = krate
                .compute_package_id_for_crate_id(aliased_summary.crate_id, self)
                .map_err(|e| CannotGetCrateData {
                    package_spec: aliased_summary.crate_id.to_string(),
                    source: Arc::new(e),
                })?;
            let aliased_krate = self.get_or_compute_crate_by_package_id(&aliased_package_id)?;
            let Ok(aliased_type_id) =
                aliased_krate.get_item_id_by_path(&aliased_summary.path, self)?
            else {
                return Ok(Err(UnknownItemPath {
                    path: aliased_summary.path.clone(),
                }
                .into()));
            };
            let aliased_item = self.get_item_by_global_type_id(&aliased_type_id);

            let new_path = {
                let path_args = &resolved_path.segments.last().unwrap().generic_arguments;
                let alias_generics = &type_alias.generics.params;
                let mut name2path_arg = GenericBindings::default();
                for (path_arg, alias_generic) in path_args.iter().zip(alias_generics.iter()) {
                    match path_arg {
                        FQGenericArgument::Type(t) => {
                            let t = resolve_fq_path_type(t, self).unwrap();
                            name2path_arg.types.insert(alias_generic.name.clone(), t);
                        }
                        FQGenericArgument::Lifetime(l) => {
                            name2path_arg
                                .lifetimes
                                .insert(alias_generic.name.clone(), l.to_binding_name());
                        }
                    }
                }

                let aliased = resolve_type(
                    &type_alias.type_,
                    type_id.package_id(),
                    self,
                    &name2path_arg,
                )
                .unwrap();
                let aliased: FQPathType = aliased.into();
                let FQPathType::ResolvedPath(aliased_path) = aliased else {
                    unreachable!();
                };
                (*aliased_path.path).clone()
            };

            // Update the loop variables to reflect alias resolution.
            type_id = aliased_type_id;
            item = aliased_item;
            krate = aliased_krate;
            resolved_path = new_path;
        }

        let resolved_item = ResolvedItem {
            item,
            item_id: type_id,
        };
        Ok(Ok((resolved_path, resolved_item)))
    }
}

impl<I: CrateIndexer> CrateRegistry for CrateCollection<I> {
    fn package_graph(&self) -> &PackageGraph {
        &self.package_graph
    }

    fn get_or_compute_crate(&self, package_id: &PackageId) -> Result<&Crate, CannotGetCrateData> {
        self.get_or_compute_crate_by_package_id(package_id)
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

#[derive(Debug, Clone)]
pub struct ResolvedItem<'a> {
    pub item: Cow<'a, Item>,
    pub item_id: GlobalItemId,
}

#[derive(thiserror::Error, Debug)]
pub enum GetItemByResolvedPathError {
    #[error(transparent)]
    UnknownItemPath(UnknownItemPath),
    #[error(transparent)]
    UnsupportedItemKind(UnsupportedItemKind),
}

impl From<UnsupportedItemKind> for GetItemByResolvedPathError {
    fn from(value: UnsupportedItemKind) -> Self {
        Self::UnsupportedItemKind(value)
    }
}

impl From<UnknownItemPath> for GetItemByResolvedPathError {
    fn from(value: UnknownItemPath) -> Self {
        Self::UnknownItemPath(value)
    }
}

#[derive(thiserror::Error, Debug)]
pub struct UnsupportedItemKind {
    pub path: Vec<String>,
    pub kind: String,
}

impl std::fmt::Display for UnsupportedItemKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let path = self.path.join("::").replace(' ', "");
        write!(
            f,
            "'{path}' pointed at {} item. I don't know how to handle that (yet)",
            self.kind
        )
    }
}
