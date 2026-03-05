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
use crate::diagnostic::DiagnosticSink;
use crate::language::{FQGenericArgument, FQPathType, UnknownCrate, krate2package_id};
use crate::rustdoc::CannotGetCrateData;
use crate::rustdoc::{ALLOC_PACKAGE_ID, CORE_PACKAGE_ID, STD_PACKAGE_ID};
use rustdoc_ext::RustdocKindExt;
use rustdoc_processor::compute_crate_docs;

use super::super::AnnotatedItem;
use super::super::annotations::{AnnotatedItems, AnnotationCoordinates};
use super::super::compute::{CacheEntryExt, RustdocCacheKey, RustdocGlobalFsCache};
use super::super::progress_reporter::ShellProgress;
use rustdoc_processor::{Crate, CrateRegistry, GlobalItemId, UnknownItemPath};

use rayon::iter::IntoParallelRefIterator;
use rustdoc_processor::CacheEntry;

/// The main entrypoint for accessing the documentation of the crates
/// in a specific `PackageGraph`.
///
/// It takes care of:
/// - Computing and caching the JSON documentation for crates in the graph;
/// - Execute queries that span the documentation of multiple crates (e.g. following crate
///   re-exports or star re-exports).
pub struct CrateCollection {
    package_id2krate: FrozenMap<PackageId, Box<Crate>>,
    /// Pavex-specific annotations extracted from each crate, stored as a side map
    /// to keep `Crate` itself Pavex-agnostic.
    annotated_items: FrozenMap<PackageId, Box<AnnotatedItems>>,
    pub(super) package_graph: PackageGraph,
    disk_cache: RustdocGlobalFsCache,
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
    /// A handle pointing at the diagnostic sink for this compilation attempt.
    diagnostic_sink: DiagnosticSink,
}

impl std::fmt::Debug for CrateCollection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CrateCollection")
            .field("package_graph", &self.package_graph)
            .field("disk_cache", &self.disk_cache)
            .finish()
    }
}

impl CrateCollection {
    /// A crate collection is specific to a workspace, as it relates to its package graph.
    ///
    /// # Project fingerprint
    ///
    /// The project fingerprint is meant to uniquely identify the current project (i.e. the current
    /// blueprint). It is used to cache the access log, the set of crates that are accessed
    /// while processing the blueprint of an application.
    /// This is then used to pre-compute/eagerly retrieve from the cache the docs for these crates
    /// the next time we process a blueprint for the same project.
    ///
    /// If the fingerprint ends up being the same for two different projects, the cache will be
    /// shared between them, which may lead to unnecessary doc loads but shouldn't cause any
    /// correctness issues.
    pub fn new(
        toolchain_name: String,
        package_graph: PackageGraph,
        project_fingerprint: String,
        cache_workspace_package_docs: bool,
        diagnostic_sink: DiagnosticSink,
    ) -> Result<Self, anyhow::Error> {
        let disk_cache = RustdocGlobalFsCache::new(
            &toolchain_name,
            cache_workspace_package_docs,
            &package_graph,
        )?;
        Ok(Self {
            package_id2krate: FrozenMap::new(),
            annotated_items: FrozenMap::new(),
            package_graph,
            diagnostic_sink,
            disk_cache,
            project_fingerprint,
            access_log: FrozenMap::new(),
            toolchain_name,
        })
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
    pub fn bootstrap_collection<I>(&self, extra_package_ids: I) -> Result<(), anyhow::Error>
    where
        I: Iterator<Item = PackageId>,
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

    /// Compute the documentation for multiple crates given their [`PackageId`]s.
    ///
    /// They won't be computed again if they are already in [`CrateCollection`]'s internal cache.
    #[tracing::instrument(skip_all, level = "trace")]
    pub fn batch_compute_crates<I>(&self, package_ids: I) -> Result<(), anyhow::Error>
    where
        I: Iterator<Item = PackageId>,
    {
        fn get_if_cached(
            package_id: PackageId,
            package_graph: &PackageGraph,
            cache: &RustdocGlobalFsCache,
            diagnostic_sink: &DiagnosticSink,
        ) -> (PackageId, Option<(Crate, AnnotatedItems)>) {
            let cache_key = RustdocCacheKey::new(&package_id, package_graph);
            match cache.get(&cache_key, package_graph) {
                Ok(None) => (package_id, None),
                Ok(Some(entry)) => (
                    package_id.clone(),
                    Some(entry.process(package_id, diagnostic_sink)),
                ),
                Err(e) => {
                    log_error!(
                        *e,
                        level: tracing::Level::WARN,
                        package_id = package_id.repr(),
                        "Failed to retrieve the documentation from the on-disk cache",
                    );
                    (package_id, None)
                }
            }
        }

        let missing_ids = package_ids
            // First check if we already have the crate docs in the in-memory cache.
            .filter(|package_id| self.get_crate_by_package_id(package_id).is_none())
            .collect::<BTreeSet<_>>();

        // It can take a while to deserialize the JSON docs for a crate from the cache,
        // so we parallelize the operation.
        let package_graph = &self.package_graph;
        let cache = &self.disk_cache;
        let sink = &self.diagnostic_sink;
        let tracing_span = Span::current();
        let map_op =
            move |id| tracing_span.in_scope(|| get_if_cached(id, package_graph, cache, sink));

        let mut to_be_computed = vec![];

        use rayon::prelude::{IntoParallelIterator, ParallelIterator};
        for (package_id, cached) in missing_ids.into_par_iter().map(map_op).collect::<Vec<_>>() {
            if let Some((krate, annotations)) = cached {
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
        let package_graph = self.package_graph();
        let diagnostic_sink = &self.diagnostic_sink;
        let indexed_krates = results
            .into_par_iter()
            .map(move |(package_id, krate)| {
                let n_diagnostics = diagnostic_sink.len();
                let (krate, annotations) =
                    super::krate::index_raw(krate, package_id.to_owned(), diagnostic_sink);

                // No issues arose in the indexing phase.
                // Let's make sure to store them in the on-disk cache for next time.
                //
                // TODO: Since we're indexing in parallel, the counter may have been incremented
                //  by a different thread, signaling an issue with indexes for another crate.
                //  It'd be enough to keep a thread-local counter to get an accurate yes/no,
                //  but since we don't get false negatives it isn't a big deal.
                let cache_indexes = n_diagnostics == diagnostic_sink.len();
                (package_id, krate, annotations, cache_indexes)
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
                let (krate, annotations) = entry.process(package_id.clone(), &self.diagnostic_sink);
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

        let n_diagnostics = self.diagnostic_sink.len();
        let (krate, annotations) =
            super::krate::index_raw(krate, package_id.to_owned(), &self.diagnostic_sink);

        // No issues arose in the indexing phase.
        // Let's make sure to store them in the on-disk cache for next time.
        let cache_indexes = n_diagnostics == self.diagnostic_sink.len();
        if let Err(e) = self.disk_cache.convert_and_insert(
            &cache_key,
            &krate,
            &annotations,
            cache_indexes,
            &self.package_graph,
        ) {
            log_error!(
                *e,
                level: tracing::Level::WARN,
                package_id = package_id.repr(),
                "Failed to store the computed JSON docs in the on-disk cache",
            );
        }

        self.annotated_items
            .insert(package_id.to_owned(), Box::new(annotations));
        self.package_id2krate
            .insert(package_id.to_owned(), Box::new(krate));
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
    pub fn get_annotated_items(&self, package_id: &PackageId) -> Option<&AnnotatedItems> {
        self.annotated_items.get(package_id)
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

    /// Retrieve type information given its [`GlobalItemId`].
    ///
    /// It panics if no item is found for the specified [`GlobalItemId`].
    pub fn get_item_by_global_type_id(&self, type_id: &GlobalItemId) -> Cow<'_, Item> {
        // Safe to unwrap since the package id is coming from a GlobalItemId.
        let krate = self.get_crate_by_package_id(&type_id.package_id).unwrap();
        krate.get_item_by_local_type_id(&type_id.rustdoc_item_id)
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
                            let t = t.resolve(self).unwrap();
                            name2path_arg.types.insert(alias_generic.name.clone(), t);
                        }
                        FQGenericArgument::Lifetime(l) => {
                            let l = match l {
                                crate::language::ResolvedPathLifetime::Named(n) => n,
                                crate::language::ResolvedPathLifetime::Static => "static",
                            }
                            .to_owned();
                            name2path_arg
                                .lifetimes
                                .insert(alias_generic.name.clone(), l);
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

    /// Retrieve the canonical path for a struct, enum or function given its [`GlobalItemId`].
    ///
    /// It panics if no item is found for the specified [`GlobalItemId`].
    pub fn get_canonical_path_by_global_type_id(
        &self,
        type_id: &GlobalItemId,
    ) -> Result<&[String], anyhow::Error> {
        // Safe to unwrap since the package id is coming from a GlobalItemId.
        let krate = self.get_crate_by_package_id(&type_id.package_id).unwrap();
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
            let used_by_krate = self.get_or_compute_crate_by_package_id(used_by_package_id)?;
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
        let definition_krate = self.get_or_compute_crate_by_package_id(&definition_package_id)?;
        let type_id = definition_krate.get_item_id_by_path(&path, self)??;
        let canonical_path = self.get_canonical_path_by_global_type_id(&type_id)?;
        Ok((type_id.clone(), canonical_path))
    }
}

impl CrateRegistry for CrateCollection {
    fn package_graph(&self) -> &PackageGraph {
        &self.package_graph
    }

    fn get_or_compute_crate(&self, package_id: &PackageId) -> Result<&Crate, CannotGetCrateData> {
        self.get_or_compute_crate_by_package_id(package_id)
    }
}

impl Drop for CrateCollection {
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
