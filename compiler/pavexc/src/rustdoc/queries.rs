use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::sync::Arc;

use ahash::{HashMap, HashSet, HashSetExt};
use anyhow::{Context, anyhow};
use elsa::FrozenMap;
use guppy::graph::PackageGraph;
use guppy::{PackageId, Version};
use indexmap::IndexSet;
use rayon::iter::IntoParallelRefIterator;
use rkyv::collections::swiss_table::ArchivedHashMap;
use rkyv::rancor::Panic;
use rkyv::string::ArchivedString;
use rkyv::util::AlignedVec;
use rkyv::vec::ArchivedVec;
use rustc_hash::FxHashMap;
use rustdoc_types::{
    ArchivedId, ArchivedItem, ExternalCrate, Item, ItemEnum, ItemKind, ItemSummary, Visibility,
};
use tracing::Span;
use tracing_log_error::log_error;

use crate::compiler::resolvers::{GenericBindings, resolve_type};
use crate::diagnostic::DiagnosticSink;
use crate::language::{FQGenericArgument, FQPathType, UnknownCrate, krate2package_id};
use crate::rustdoc::compute::CacheEntry;
use crate::rustdoc::version_matcher::VersionMatcher;
use crate::rustdoc::{ALLOC_PACKAGE_ID, CORE_PACKAGE_ID, STD_PACKAGE_ID};
use crate::rustdoc::{CannotGetCrateData, TOOLCHAIN_CRATES, utils};

use super::AnnotatedItem;
use super::annotations::{
    self, AnnotatedItems, AnnotationCoordinates, QueueItem, invalid_diagnostic_attribute,
    parse_pavex_attributes,
};
use super::compute::{RustdocCacheKey, RustdocGlobalFsCache, compute_crate_docs};

/// The main entrypoint for accessing the documentation of the crates
/// in a specific `PackageGraph`.
///
/// It takes care of:
/// - Computing and caching the JSON documentation for crates in the graph;
/// - Execute queries that span the documentation of multiple crates (e.g. following crate
///   re-exports or star re-exports).
pub struct CrateCollection {
    package_id2krate: FrozenMap<PackageId, Box<Crate>>,
    package_graph: PackageGraph,
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
        ) -> (PackageId, Option<Crate>) {
            let cache_key = RustdocCacheKey::new(&package_id, &package_graph);
            match cache.get(&cache_key, &package_graph) {
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
            move |id| tracing_span.in_scope(|| get_if_cached(id, &package_graph, cache, &sink));

        let mut to_be_computed = vec![];

        use rayon::prelude::{IntoParallelIterator, ParallelIterator};
        for (package_id, cached) in missing_ids.into_par_iter().map(map_op).collect::<Vec<_>>() {
            if let Some(krate) = cached {
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
                let krate = Crate::index_raw(krate, package_id.to_owned(), diagnostic_sink);

                // No issues arose in the indexing phase.
                // Let's make sure to store them in the on-disk cache for next time.
                //
                // TODO: Since we're indexing in parallel, the counter may have been incremented
                //  by a different thread, signaling an issue with indexes for another crate.
                //  It'd be enough to keep a thread-local counter to get an accurate yes/no,
                //  but since we don't get false negatives it isn't a big deal.
                let cache_indexes = n_diagnostics == diagnostic_sink.len();
                (package_id, krate, cache_indexes)
            })
            .collect::<Vec<_>>();
        // Then conversion to the desired cache format:
        let mut cache_entries: HashMap<_, _> = indexed_krates
            .par_iter()
            .filter_map(|(package_id, krate, cache_indexes)| {
                let data = if *cache_indexes {
                    CacheEntry::new(&krate)
                } else {
                    CacheEntry::raw(&krate)
                };
                let cache_key = RustdocCacheKey::new(&package_id, package_graph);
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
        for (package_id, _, _) in &indexed_krates {
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

        for (package_id, krate, _) in indexed_krates {
            if to_be_inserted.contains(&package_id) {
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
            Ok(Some(krate)) => {
                let krate = krate.process(package_id.clone(), &self.diagnostic_sink);
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
        )
        .map_err(|e| CannotGetCrateData {
            package_spec: package_id.to_string(),
            source: Arc::new(e),
        })?
        .remove(package_id)
        .unwrap();

        let n_diagnostics = self.diagnostic_sink.len();
        let krate = Crate::index_raw(krate, package_id.to_owned(), &self.diagnostic_sink);

        // No issues arose in the indexing phase.
        // Let's make sure to store them in the on-disk cache for next time.
        let cache_indexes = n_diagnostics == self.diagnostic_sink.len();
        if let Err(e) = self.disk_cache.convert_and_insert(
            &cache_key,
            &krate,
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

    /// Retrieve the annotation associated with the given item, if any.
    pub fn annotation(&self, item_id: &GlobalItemId) -> Option<&AnnotatedItem> {
        let krate = self.get_crate_by_package_id(&item_id.package_id)?;
        krate
            .annotated_items
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
        Ok(Ok(krate
            .annotated_items
            .get_by_annotation_id(&c.id)
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

/// Thin wrapper around [`rustdoc_types::Crate`] to:
/// - bundle derived indexes;
/// - provide query helpers with good error messages.
///
/// It also records the `PackageId` for the corresponding crate within the dependency tree
/// for the workspace it belongs to.
#[derive(Debug, Clone)]
pub struct Crate {
    pub(crate) core: CrateCore,
    /// An index to lookup the id of a type given one of its import paths, either
    /// public or private.
    ///
    /// The index does NOT contain macros, since macros and types live in two
    /// different namespaces and can contain items with the same name.
    /// E.g. `core::clone::Clone` is both a trait and a derive macro.
    pub(super) import_path2id: ImportPath2Id,
    /// Types (or modules!) re-exported from other crates.
    pub(crate) external_re_exports: ExternalReExports,
    /// All the items in this crate that have been annotated with an attribute from the `diagnostic::pavex::*` namespace.
    pub(crate) annotated_items: AnnotatedItems,
    /// An in-memory index of all modules, traits, structs, enums, and functions that were defined in the current crate.
    ///
    /// It can be used to retrieve all publicly visible items as well as computing a "canonical path"
    /// for each of them.
    pub(crate) import_index: ImportIndex,
    /// An internal cache to avoid traversing the package graph every time we need to
    /// translate a crate id into a package id via [`Self::compute_package_id_for_crate_id`]
    /// or [`Self::compute_package_id_for_crate_id_with_hint`].
    pub(super) crate_id2package_id:
        Arc<std::sync::RwLock<HashMap<(u32, Option<String>), PackageId>>>,
}

#[derive(Debug, Clone)]
/// An index to lookup the id of a type given one of its import paths, either
/// public or private.
///
/// The index does NOT contain macros, since macros and types live in two
/// different namespaces and can contain items with the same name.
/// E.g. `core::clone::Clone` is both a trait and a derive macro.
///
/// Since the index can be quite large, we try to avoid deserializing it all at once.
///
/// The `Eager` variant contains the entire index, fully deserialized. This is what we get
/// when we have had to index the documentation for the crate on the fly.
///
/// The `Lazy` variant contains the index as a byte array, with entries deserialized on demand.
pub(crate) enum ImportPath2Id {
    Eager(EagerImportPath2Id),
    Lazy(LazyImportPath2Id),
}

impl ImportPath2Id {
    pub fn get(&self, path: &[String]) -> Option<rustdoc_types::Id> {
        match self {
            ImportPath2Id::Eager(m) => m.0.get(path).cloned(),
            ImportPath2Id::Lazy(m) => m.get_deserialized(path),
        }
    }
}

#[derive(Debug, Clone)]
/// See [`ImportPath2Id`] for more information.
pub(crate) struct EagerImportPath2Id(pub HashMap<Vec<String>, rustdoc_types::Id>);

/// See [`ImportPath2Id`] for more information.
///
/// Stores rkyv-serialized bytes of a `HashMap<Vec<String>, Id>` and provides zero-copy access.
#[derive(Debug, Clone)]
pub(crate) struct LazyImportPath2Id(pub AlignedVec);

impl LazyImportPath2Id {
    #[inline]
    fn archived(&self) -> &ArchivedHashMap<ArchivedVec<ArchivedString>, ArchivedId> {
        unsafe {
            rkyv::access_unchecked::<ArchivedHashMap<ArchivedVec<ArchivedString>, ArchivedId>>(
                &self.0,
            )
        }
    }

    pub fn get(&self, path: &[String]) -> Option<&ArchivedId> {
        let path_vec: Vec<String> = path.to_vec();
        let bytes = rkyv::to_bytes::<Panic>(&path_vec).ok()?;

        let archived_key = unsafe { rkyv::access_unchecked::<ArchivedVec<ArchivedString>>(&bytes) };
        self.archived().get(archived_key)
    }

    pub fn get_deserialized(&self, path: &[String]) -> Option<rustdoc_types::Id> {
        let archived = self.get(path)?;
        Some(rkyv::deserialize::<_, Panic>(archived).unwrap())
    }
}

#[derive(
    Debug, Clone, Default, serde::Serialize, serde::Deserialize, bincode::Encode, bincode::Decode,
)]
/// Track re-exports of types (or entire modules!) from other crates.
pub struct ExternalReExports {
    /// Key: the path of the re-exported type in the current crate.
    /// Value: the id of the `rustdoc` item of kind `use` that performed the re-export.
    ///
    /// E.g. `pub use hyper::server as sx;` in `lib.rs` would use `vec!["my_crate", "sx"]`
    /// as key in this map.
    target_path2use_id: HashMap<Vec<String>, rustdoc_types::Id>,
    /// Key: the id of the `rustdoc` item of kind `use` that performed the re-export.
    /// Value: metadata about the re-export.
    use_id2re_export: HashMap<rustdoc_types::Id, ExternalReExport>,
}

impl ExternalReExports {
    /// Iteratore over the external re-exports that have been collected.
    pub fn iter(
        &self,
    ) -> impl Iterator<Item = (&Vec<String>, rustdoc_types::Id, &ExternalReExport)> {
        self.target_path2use_id
            .iter()
            .map(|(target_path, id)| (target_path, *id, &self.use_id2re_export[id]))
    }

    /// Add another re-export to the database.
    pub fn insert(
        &mut self,
        krate: &CrateData,
        use_item: &rustdoc_types::Item,
        current_path: &[String],
    ) {
        let ItemEnum::Use(use_) = &use_item.inner else {
            unreachable!()
        };
        let imported_id = use_.id.expect("Import doesn't have an associated id");
        let Some(imported_summary) = krate.paths.get(&imported_id) else {
            // TODO: this is firing for std's JSON docs. File a bug report.
            // panic!("The imported id ({}) is not listed in the index nor in the path section of rustdoc's JSON output", imported_id.0)
            return;
        };
        debug_assert!(imported_summary.crate_id != 0);
        // We are looking at a public re-export of another crate
        // (e.g. `pub use hyper;`), one of its modules or one of its items.
        // Due to how re-exports are handled in `rustdoc`, the re-exported
        // items inside that foreign module will not be found in the `index`
        // for this crate.
        // We intentionally add foreign items to the index to get a "complete"
        // picture of all the types available in this crate.
        let external_crate_id = imported_summary.crate_id;
        let source_path = imported_summary.path.to_owned();
        let re_exported_path = {
            let mut p = current_path.to_owned();
            if !use_.is_glob {
                p.push(use_.name.clone());
            }
            p
        };
        let re_export = ExternalReExport {
            source_path,
            external_crate_id,
        };

        self.target_path2use_id
            .insert(re_exported_path, use_item.id);
        self.use_id2re_export.insert(use_item.id, re_export);
    }

    /// Retrieve the re-exported item from the crate it was defined into.
    ///
    /// # Panics
    ///
    /// Panics if the provided `use_id` doesn't exist as a key in the re-export registry.
    pub fn get_target_item_id(
        &self,
        // The crate associated with these re-exports.
        re_exported_from: &Crate,
        krate_collection: &CrateCollection,
        use_id: rustdoc_types::Id,
    ) -> Result<Option<GlobalItemId>, CannotGetCrateData> {
        let re_export = &self.use_id2re_export[&use_id];
        let source_package_id = re_exported_from
            .core
            .compute_package_id_for_crate_id(re_export.external_crate_id, krate_collection, None)
            .expect("Failed to compute the package id for a given external crate id");
        let source_krate =
            krate_collection.get_or_compute_crate_by_package_id(&source_package_id)?;
        let Ok(Ok(source_id)) =
            source_krate.get_item_id_by_path(&re_export.source_path, krate_collection)
        else {
            return Ok(None);
        };
        Ok(Some(source_id))
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, bincode::Encode, bincode::Decode)]
/// Information about a type (or module) re-exported from another crate.
pub struct ExternalReExport {
    /// The path of the re-exported type in the crate it was re-exported from.
    ///
    /// E.g. `pub use hyper::server as sx;` in `lib.rs` would set `source_path` to
    /// `vec!["hyper", "server"]`.
    source_path: Vec<String>,
    /// The id of the source crate in the `external_crates` section of the JSON
    /// documentation of the crate that re-exported it.
    external_crate_id: u32,
}

#[derive(
    Debug, Clone, Default, serde::Serialize, serde::Deserialize, bincode::Encode, bincode::Decode,
)]
pub struct ImportIndex {
    /// A mapping that keeps track of all modules defined in the current crate.
    ///
    /// We track modules separately because their names are allowed to collide with
    /// type and function names.
    pub modules: HashMap<rustdoc_types::Id, ImportIndexEntry>,
    /// A mapping that keeps track of traits, structs, enums and functions
    /// defined in the current crate.
    pub items: HashMap<rustdoc_types::Id, ImportIndexEntry>,
    /// A mapping that associates the id of each re-export (`pub use ...`) to the id
    /// of the module it was re-exported from.
    pub re_export2parent_module: HashMap<rustdoc_types::Id, rustdoc_types::Id>,
}

/// An entry in [`ImportIndex`].
#[derive(
    Debug, Clone, Default, serde::Serialize, serde::Deserialize, bincode::Encode, bincode::Decode,
)]
pub struct ImportIndexEntry {
    /// All the public paths that can be used to import the item.
    pub public_paths: BTreeSet<SortablePath>,
    /// All the private paths that can be used to import the item.
    pub private_paths: BTreeSet<SortablePath>,
    /// The path where the item was originally defined.
    ///
    /// It may be set to `None` if we can't access the original definition.
    /// E.g. an item defined in a private module of `std`, where we only have access
    /// to the public API.
    pub defined_at: Option<Vec<String>>,
}

/// The visibility of a path inside [`ImportIndexEntry`].
pub enum EntryVisibility {
    /// The item can be imported from outside the crate where it was defined.
    Public,
    /// The item can only be imported from within the crate where it was defined.
    Private,
}

impl ImportIndexEntry {
    /// A private constructor.
    fn empty() -> Self {
        Self {
            public_paths: BTreeSet::new(),
            private_paths: BTreeSet::new(),
            defined_at: None,
        }
    }

    /// Create a new entry from a path.
    pub fn new(path: Vec<String>, visibility: EntryVisibility, is_definition: bool) -> Self {
        let mut entry = Self::empty();
        if is_definition {
            entry.defined_at = Some(path.clone());
        }
        match visibility {
            EntryVisibility::Public => entry.public_paths.insert(SortablePath(path)),
            EntryVisibility::Private => entry.private_paths.insert(SortablePath(path)),
        };
        entry
    }

    /// Add a new private path for this item.
    pub fn insert_private(&mut self, path: Vec<String>) {
        self.private_paths.insert(SortablePath(path));
    }

    /// Add a new path for this item.
    pub fn insert(&mut self, path: Vec<String>, visibility: EntryVisibility) {
        match visibility {
            EntryVisibility::Public => self.public_paths.insert(SortablePath(path)),
            EntryVisibility::Private => self.private_paths.insert(SortablePath(path)),
        };
    }

    /// Types can be exposed under multiple paths.
    /// This method returns a "canonical" importable path—i.e. the shortest importable path
    /// pointing at the type you specified.
    ///
    /// If the type is public, this method returns the shortest public path.
    /// If the type is private, this method returns the shortest private path.
    pub fn canonical_path(&self) -> &[String] {
        if let Some(SortablePath(p)) = self.public_paths.first() {
            return p;
        }
        if let Some(SortablePath(p)) = self.private_paths.first() {
            return p;
        }
        unreachable!("There must be at least one path associated to an import index entry")
    }

    /// Returns all paths associated with the type, both public and private.
    pub fn paths(&self) -> impl Iterator<Item = &[String]> {
        self.public_paths
            .iter()
            .map(|SortablePath(p)| p.as_slice())
            .chain(
                self.private_paths
                    .iter()
                    .map(|SortablePath(p)| p.as_slice()),
            )
    }
}

#[derive(
    Debug,
    Clone,
    Eq,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
    bincode::Encode,
    bincode::Decode,
)]
#[serde(transparent)]
pub struct SortablePath(pub Vec<String>);

impl Ord for SortablePath {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.0.len().cmp(&other.0.len()) {
            // Compare lexicographically if lengths are equal
            Ordering::Equal => self.0.cmp(&other.0),
            other => other,
        }
    }
}

impl PartialOrd for SortablePath {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone)]
pub(crate) struct CrateCore {
    /// The `PackageId` for the corresponding crate within the dependency tree
    /// for the workspace it belongs to.
    pub(crate) package_id: PackageId,
    /// The JSON documentation for the crate.
    pub(super) krate: CrateData,
}

#[derive(Debug, Clone)]
/// The JSON documentation for a crate.
pub(crate) struct CrateData {
    /// The id of the root item for the crate.
    pub root_item_id: rustdoc_types::Id,
    /// A mapping from the id of an external crate to the information about it.
    #[allow(clippy::disallowed_types)]
    pub external_crates: FxHashMap<u32, ExternalCrate>,
    /// A mapping from the id of a type to its fully qualified path.
    /// Primarily useful for foreign items that are being re-exported by this crate.
    #[allow(clippy::disallowed_types)]
    pub paths: FxHashMap<rustdoc_types::Id, ItemSummary>,
    /// The version of the JSON format used by rustdoc.
    pub format_version: u32,
    /// The index of all the items in the crate.
    pub index: CrateItemIndex,
}
#[derive(Debug, Clone)]
/// The index of all the items in the crate.
///
/// Since the index can be quite large, we try to avoid deserializing it all at once.
///
/// The `Eager` variant contains the entire index, fully deserialized. This is what we get
/// when we have had to compute the documentation for the crate on the fly.
///
/// The `Lazy` variant contains the index as a byte array. There is a mapping from the
/// id of an item to the start and end index of the item's bytes in the byte array.
/// We can therefore deserialize the item only if we need to access it.
/// Since we only access a tiny portion of the items in the index (especially for large crates),
/// this translates in a significant performance improvement.
pub(crate) enum CrateItemIndex {
    Eager(EagerCrateItemIndex),
    Lazy(LazyCrateItemIndex),
}

impl CrateItemIndex {
    /// Retrieve an item from the index given its id.
    pub fn get(&self, id: &rustdoc_types::Id) -> Option<Cow<'_, Item>> {
        match self {
            Self::Eager(index) => index.index.get(id).map(Cow::Borrowed),
            Self::Lazy(index) => {
                let item = index.get_deserialized(id)?;
                Some(Cow::Owned(item))
            }
        }
    }
}

#[derive(Debug, Clone)]
/// See [`CrateItemIndex`] for more information.
pub(crate) struct EagerCrateItemIndex {
    #[allow(clippy::disallowed_types)]
    pub index: FxHashMap<rustdoc_types::Id, Item>,
}

/// See [`CrateItemIndex`] for more information.
///
/// Stores rkyv-serialized bytes of a `HashMap<Id, Item>` and provides zero-copy access.
#[derive(Debug, Clone)]
pub(crate) struct LazyCrateItemIndex {
    /// The rkyv-serialized bytes containing a `HashMap<Id, Item>`.
    pub(super) bytes: AlignedVec,
}

impl LazyCrateItemIndex {
    /// Get zero-copy access to the archived HashMap.
    #[inline]
    fn archived(&self) -> &ArchivedHashMap<ArchivedId, ArchivedItem> {
        // SAFETY: The bytes were serialized by rkyv from a valid HashMap<Id, Item>.
        // We trust the cache to contain valid data.
        unsafe { rkyv::access_unchecked::<ArchivedHashMap<ArchivedId, ArchivedItem>>(&self.bytes) }
    }

    /// Get an item by its ID, returning a reference to the archived item.
    pub fn get(&self, id: &rustdoc_types::Id) -> Option<&ArchivedItem> {
        self.archived().get(&ArchivedId(id.0.into()))
    }

    /// Deserialize an item by its ID.
    pub fn get_deserialized(&self, id: &rustdoc_types::Id) -> Option<Item> {
        let archived = self.get(id)?;
        Some(rkyv::deserialize::<Item, Panic>(archived).unwrap())
    }
}

impl CrateCore {
    /// Given a crate id, return the corresponding [`PackageId`].
    ///
    /// # Disambiguation
    ///
    /// There might be multiple crates in the dependency graph with the same name, causing
    /// disambiguation issues.
    /// To help out, you can specify `maybe_dependent`: the name of a crate that you think
    /// depends on the crate you're trying to resolve.
    /// This can narrow down the portion of the dependency graph that we need to search,
    /// thus removing ambiguity.
    ///
    /// # Panics
    ///
    /// It panics if the provided crate id doesn't appear in the JSON documentation
    /// for this crate—i.e. if it's not `0` or assigned to one of its transitive dependencies.
    pub fn compute_package_id_for_crate_id(
        &self,
        crate_id: u32,
        collection: &CrateCollection,
        maybe_dependent_crate_name: Option<&str>,
    ) -> Result<PackageId, anyhow::Error> {
        compute_package_id_for_crate_id(
            &self.package_id,
            &self.krate.external_crates,
            crate_id,
            maybe_dependent_crate_name,
            &collection.package_graph,
        )
    }
}

impl Crate {
    pub(super) fn index_raw(
        krate: rustdoc_types::Crate,
        package_id: PackageId,
        diagnostics: &DiagnosticSink,
    ) -> Self {
        let crate_data = CrateData {
            root_item_id: krate.root,
            index: CrateItemIndex::Eager(EagerCrateItemIndex { index: krate.index }),
            external_crates: krate.external_crates,
            format_version: krate.format_version,
            paths: krate.paths,
        };
        Self::index(crate_data, package_id, diagnostics)
    }

    #[tracing::instrument(skip_all, name = "index_crate_docs", fields(package.id = package_id.repr()))]
    pub(super) fn index(
        krate: CrateData,
        package_id: PackageId,
        diagnostics: &DiagnosticSink,
    ) -> Self {
        let mut import_path2id: HashMap<_, _> = krate
            .paths
            .iter()
            .filter_map(|(id, summary)| {
                // We only want types, no macros
                if matches!(summary.kind, ItemKind::Macro | ItemKind::ProcDerive) {
                    return None;
                }
                // We will index local items on our own.
                // We don't get them from `paths` because it may include private items
                // as well, and we don't have a way to figure out if an item is private
                // or not from the summary info.
                if summary.crate_id == 0 {
                    return None;
                }

                Some((summary.path.clone(), id.to_owned()))
            })
            .collect();

        let mut annotation_queue = BTreeSet::<QueueItem>::new();
        let mut import_index = ImportIndex::default();
        let mut external_re_exports = Default::default();
        index_local_types(
            &krate,
            &package_id,
            IndexSet::new(),
            vec![],
            &mut import_index,
            &mut external_re_exports,
            &mut annotation_queue,
            &krate.root_item_id,
            true,
            None,
            false,
            diagnostics,
        );

        import_path2id.reserve(import_index.items.len());
        for (id, entry) in import_index.items.iter() {
            for path in entry.public_paths.iter().chain(entry.private_paths.iter()) {
                if !import_path2id.contains_key(&path.0) {
                    import_path2id.insert(path.0.clone(), id.to_owned());
                }
            }
        }

        let mut self_ = Self {
            core: CrateCore { package_id, krate },
            import_path2id: ImportPath2Id::Eager(EagerImportPath2Id(import_path2id)),
            import_index,
            external_re_exports,
            annotated_items: AnnotatedItems::default(),
            crate_id2package_id: Default::default(),
        };

        let annotated_items = annotations::process_queue(annotation_queue, &self_, diagnostics);
        self_.annotated_items = annotated_items;
        self_
    }

    /// The name of the crate.
    pub fn crate_name(&self) -> String {
        self.core
            .krate
            .index
            .get(&self.core.krate.root_item_id)
            .as_ref()
            .expect("Can't find the crate root")
            .name
            .clone()
            .expect("The crate root doesn't have a name")
    }

    pub fn crate_version<'a>(&self, package_graph: &'a PackageGraph) -> &'a semver::Version {
        let metadata = package_graph.metadata(&self.core.package_id).unwrap();
        metadata.version()
    }

    /// Given a crate id, return the corresponding [`PackageId`].
    ///
    /// It panics if the provided crate id doesn't appear in the JSON documentation
    /// for this crate—i.e. if it's not `0` or assigned to one of its transitive dependencies.
    pub fn compute_package_id_for_crate_id(
        &self,
        crate_id: u32,
        collection: &CrateCollection,
    ) -> Result<PackageId, anyhow::Error> {
        self.compute_package_id_for_crate_id_with_hint(crate_id, collection, None)
    }

    /// Given a crate id, return the corresponding [`PackageId`].
    ///
    /// # Disambiguation
    ///
    /// There might be multiple crates in the dependency graph with the same name, causing
    /// disambiguation issues.
    /// To help out, you can specify `maybe_dependent`: the name of a crate that you think
    /// depends on the crate you're trying to resolve.
    /// This can narrow down the portion of the dependency graph that we need to search,
    /// thus removing ambiguity.
    ///
    /// # Panics
    ///
    /// It panics if the provided crate id doesn't appear in the JSON documentation
    /// for this crate—i.e. if it's not `0` or assigned to one of its transitive dependencies.
    pub fn compute_package_id_for_crate_id_with_hint(
        &self,
        crate_id: u32,
        collection: &CrateCollection,
        maybe_dependent_crate_name: Option<&str>,
    ) -> Result<PackageId, anyhow::Error> {
        // Check the cache first.
        if let Some(package_id) = self
            .crate_id2package_id
            .read()
            .unwrap()
            .get(&(crate_id, maybe_dependent_crate_name.map(|s| s.to_owned())))
        {
            return Ok(package_id.to_owned());
        }

        // If we don't have a cached entry, perform the graph traversal.
        let outcome = self.core.compute_package_id_for_crate_id(
            crate_id,
            collection,
            maybe_dependent_crate_name,
        );

        // If successful, cache the outcome.
        if let Ok(outcome) = &outcome {
            self.crate_id2package_id.write().unwrap().insert(
                (crate_id, maybe_dependent_crate_name.map(|s| s.to_owned())),
                outcome.to_owned(),
            );
        }
        outcome
    }

    pub fn get_item_id_by_path(
        &self,
        path: &[String],
        krate_collection: &CrateCollection,
    ) -> Result<Result<GlobalItemId, UnknownItemPath>, CannotGetCrateData> {
        if let Some(id) = self.import_path2id.get(path) {
            return Ok(Ok(GlobalItemId::new(id, self.core.package_id.to_owned())));
        }

        for (
            re_exported_path_prefix,
            _,
            ExternalReExport {
                source_path: source_path_prefix,
                external_crate_id,
            },
        ) in self.external_re_exports.iter()
        {
            if re_exported_path_prefix
                .iter()
                .zip(path)
                .all(|(a, b)| a == b)
            {
                let mut original_source_path = source_path_prefix.clone();
                for segment in path.iter().skip(re_exported_path_prefix.len()) {
                    original_source_path.push(segment.to_owned());
                }

                let source_package_id = self
                    .core
                    .compute_package_id_for_crate_id(*external_crate_id, krate_collection, None)
                    .unwrap();
                let source_krate = krate_collection
                    .get_or_compute_crate_by_package_id(&source_package_id)
                    .unwrap();
                if let Ok(source_id) =
                    source_krate.get_item_id_by_path(&original_source_path, krate_collection)
                {
                    return Ok(source_id);
                }
            }
        }

        Ok(Err(UnknownItemPath {
            path: path.to_owned(),
        }))
    }

    /// Return the crate_id, path and item kind for a **local** type id.
    ///
    /// It only works for structs, enums and functions.
    /// It **will** fail if the id points to a method!
    fn get_summary_by_local_type_id(
        &self,
        id: &rustdoc_types::Id,
    ) -> Result<&rustdoc_types::ItemSummary, anyhow::Error> {
        self.core.krate.paths.get(id).ok_or_else(|| {
            anyhow!(
                "Failed to look up the type id `{}` in the rustdoc's path index for `{}`. \
                This is likely to be a bug in rustdoc's JSON output.",
                id.0,
                self.core.package_id.repr()
            )
        })
    }

    pub fn get_item_by_local_type_id(&self, id: &rustdoc_types::Id) -> Cow<'_, Item> {
        let type_ = self.maybe_get_item_by_local_type_id(id);
        if type_.is_none() {
            panic!(
                "Failed to look up the type id `{}` in the rustdoc's index for package `{}`.",
                id.0,
                self.core.package_id.repr()
            )
        }
        type_.unwrap()
    }

    /// Same as `get_type_by_local_type_id`, but returns `None` instead of panicking
    /// if the type is not found.
    pub fn maybe_get_item_by_local_type_id(&self, id: &rustdoc_types::Id) -> Option<Cow<'_, Item>> {
        self.core.krate.index.get(id)
    }

    /// Types can be exposed under multiple paths.
    /// This method returns a "canonical" importable path—i.e. the shortest importable path
    /// pointing at the type you specified.
    fn get_canonical_path(&self, type_id: &GlobalItemId) -> Result<&[String], anyhow::Error> {
        if type_id.package_id == self.core.package_id
            && let Some(entry) = self.import_index.items.get(&type_id.rustdoc_item_id)
        {
            return Ok(entry.canonical_path());
        }
        Err(anyhow::anyhow!(
            "Failed to find an importable path for the type id `{:?}` in the index I computed for `{:?}`. \
            This is likely to be a bug in pavex's handling of rustdoc's JSON output or in rustdoc itself.",
            type_id,
            self.core.package_id.repr()
        ))
    }
}

fn index_local_types<'a>(
    krate: &'a CrateData,
    package_id: &'a PackageId,
    // The ordered set of modules we navigated to reach this item.
    // It used to detect infinite loops.
    mut navigation_history: IndexSet<rustdoc_types::Id>,
    mut current_path: Vec<String>,
    import_index: &mut ImportIndex,
    re_exports: &mut ExternalReExports,
    annotation_queue: &mut BTreeSet<QueueItem>,
    current_item_id: &rustdoc_types::Id,
    is_public: bool,
    // Set when the current item has been re-exported via a `use` statement
    // that includes an `as` rename.
    renamed_to: Option<String>,
    // If `true`, we've encountered at least a `pub use`/`use` statement while
    // navigating to this item.
    encountered_use: bool,
    diagnostics: &DiagnosticSink,
) {
    // TODO: the way we handle `current_path` is extremely wasteful,
    //       we can likely reuse the same buffer throughout.
    let current_item = match krate.index.get(current_item_id) {
        None => {
            if let Some(summary) = krate.paths.get(current_item_id)
                && summary.kind == ItemKind::Primitive
            {
                // This is a known bug—see https://github.com/rust-lang/rust/issues/104064
                return;
            }
            panic!(
                "Failed to retrieve item id `{:?}` from the JSON `index` for package id `{}`.",
                &current_item_id,
                package_id.repr()
            )
        }
        Some(i) => i,
    };

    match parse_pavex_attributes(&current_item.attrs) {
        Ok(Some(_)) => {
            annotation_queue.insert(QueueItem::Standalone(*current_item_id));
        }
        Ok(None) => {}
        Err(e) => {
            // TODO: Only report an error if it's a crate from the current workspace
            invalid_diagnostic_attribute(e, current_item.as_ref(), diagnostics);
        }
    };

    let is_public = is_public && current_item.visibility == Visibility::Public;

    let mut add_to_import_index = |path: Vec<String>, is_module: bool| {
        let visibility = if is_public {
            EntryVisibility::Public
        } else {
            EntryVisibility::Private
        };
        let is_definition = !encountered_use;
        let index = if is_module {
            &mut import_index.modules
        } else {
            &mut import_index.items
        };
        match index.get_mut(current_item_id) {
            Some(entry) => {
                entry.insert(path.clone(), visibility);
                if is_definition {
                    entry.defined_at = Some(path);
                }
            }
            None => {
                index.insert(
                    *current_item_id,
                    ImportIndexEntry::new(path, visibility, is_definition),
                );
            }
        }
    };

    let current_item = current_item.as_ref();
    match &current_item.inner {
        ItemEnum::Module(m) => {
            let current_path_segment = renamed_to.unwrap_or_else(|| {
                current_item
                    .name
                    .as_deref()
                    .expect("All 'module' items have a 'name' property")
                    .to_owned()
            });
            current_path.push(current_path_segment);

            add_to_import_index(
                current_path
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>(),
                true,
            );

            navigation_history.insert(*current_item_id);
            for item_id in &m.items {
                index_local_types(
                    krate,
                    package_id,
                    navigation_history.clone(),
                    current_path.clone(),
                    import_index,
                    re_exports,
                    annotation_queue,
                    item_id,
                    is_public,
                    None,
                    encountered_use,
                    diagnostics,
                );
            }
        }
        ItemEnum::Use(i) => {
            let Some(imported_id) = &i.id else {
                return;
            };

            import_index
                .re_export2parent_module
                .insert(current_item.id, *navigation_history.last().unwrap());

            let Some(imported_item) = krate.index.get(imported_id) else {
                // We are looking at a public re-export of another crate
                // (e.g. `pub use hyper;`), one of its modules or one of its items.
                // Due to how re-exports are handled in `rustdoc`, the re-exported
                // items inside that foreign module will not be found in the `index`
                // for this crate.
                // We intentionally add foreign items to the index to get a "complete"
                // picture of all the types available in this crate.
                re_exports.insert(krate, current_item, &current_path);
                return;
            };
            if let ItemEnum::Module(re_exported_module) = &imported_item.inner {
                if !i.is_glob {
                    current_path.push(i.name.clone());
                }
                // In Rust it is possible to create infinite loops with local modules!
                // Minimal example:
                // ```rust
                // pub struct A;
                // mod inner {
                //   pub use crate as b;
                // }
                // ```
                // We use this check to detect if we're about to get stuck in an infinite
                // loop, so that we can break early.
                // It does mean that some paths that _would_ be valid won't be recognised,
                // but this pattern is rarely used and for the time being we don't want to
                // take the complexity hit of making visible paths lazily evaluated.
                let infinite_loop = !navigation_history.insert(*imported_id);
                if !infinite_loop {
                    for re_exported_item_id in &re_exported_module.items {
                        index_local_types(
                            krate,
                            package_id,
                            navigation_history.clone(),
                            current_path.clone(),
                            import_index,
                            re_exports,
                            annotation_queue,
                            re_exported_item_id,
                            is_public,
                            None,
                            true,
                            diagnostics,
                        );
                    }
                }
            } else {
                navigation_history.insert(*imported_id);

                if matches!(
                    imported_item.inner,
                    ItemEnum::Enum(_)
                        | ItemEnum::Struct(_)
                        | ItemEnum::Trait(_)
                        | ItemEnum::Function(_)
                        | ItemEnum::Primitive(_)
                        | ItemEnum::TypeAlias(_)
                ) {
                    // We keep track of the source path in our indexes.
                    // This is useful, in particular, if we don't have
                    // access to the source module of the imported item.
                    // This can happen when working with `std`/`alloc`/`core`
                    // since the JSON output doesn't include private/doc-hidden
                    // items.
                    let mut normalized_source_path = vec![];
                    let source_segments = i.source.split("::");
                    for segment in source_segments {
                        if segment == "self" {
                            normalized_source_path
                                .extend(current_path.iter().map(|s| s.to_string()));
                        } else if segment == "crate" {
                            normalized_source_path.push(current_path[0].to_string())
                        } else {
                            normalized_source_path.push(segment.to_string());
                        }
                    }
                    // Assume it's private unless we find out otherwise later on
                    match import_index.items.get_mut(imported_id) {
                        Some(entry) => {
                            entry.insert_private(normalized_source_path);
                        }
                        None => {
                            import_index.items.insert(
                                *imported_id,
                                ImportIndexEntry::new(
                                    normalized_source_path,
                                    EntryVisibility::Private,
                                    false,
                                ),
                            );
                        }
                    }
                }

                index_local_types(
                    krate,
                    package_id,
                    navigation_history,
                    current_path.clone(),
                    import_index,
                    re_exports,
                    annotation_queue,
                    imported_id,
                    is_public,
                    Some(i.name.clone()),
                    true,
                    diagnostics,
                );
            }
        }
        ItemEnum::Trait(_)
        | ItemEnum::Primitive(_)
        | ItemEnum::Function(_)
        | ItemEnum::Enum(_)
        | ItemEnum::Struct(_)
        | ItemEnum::TypeAlias(_) => {
            let name = current_item.name.as_deref().expect(
                "All 'struct', 'function', 'enum', 'type_alias', 'primitive' and 'trait' items have a 'name' property",
            );
            if matches!(current_item.inner, ItemEnum::Primitive(_)) {
                // E.g. `std::bool` won't work, `std::primitive::bool` does work but the `primitive` module
                // is not visible in the JSON docs for `std`/`core`.
                // A hacky workaround, but it works.
                current_path.push("primitive".into());
            }
            current_path.push(renamed_to.unwrap_or_else(|| name.to_owned()));
            let path: Vec<_> = current_path.into_iter().map(|s| s.to_string()).collect();
            add_to_import_index(path, false);

            // Even if the item itself may not be annotated, one of its impls may be.
            annotation_queue.insert(QueueItem::Standalone(*current_item_id));
        }
        _ => {}
    }
}

/// An identifier that unequivocally points to a type within a [`CrateCollection`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GlobalItemId {
    pub(crate) rustdoc_item_id: rustdoc_types::Id,
    pub(crate) package_id: PackageId,
}

impl GlobalItemId {
    pub fn new(rustdoc_item_id: rustdoc_types::Id, package_id: PackageId) -> Self {
        Self {
            rustdoc_item_id,
            package_id,
        }
    }

    pub fn package_id(&self) -> &PackageId {
        &self.package_id
    }
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

#[derive(thiserror::Error, Debug)]
pub struct UnknownItemPath {
    pub path: Vec<String>,
}

impl std::fmt::Display for UnknownItemPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let path = self.path.join("::").replace(' ', "");
        let krate = self.path.first().unwrap();
        write!(
            f,
            "I could not find '{path}' in the auto-generated documentation for '{krate}'."
        )
    }
}

pub trait RustdocKindExt {
    /// Return a string representation of this item's kind (e.g. `a function`).
    fn kind(&self) -> &'static str;
}

impl RustdocKindExt for ItemEnum {
    fn kind(&self) -> &'static str {
        match self {
            ItemEnum::Module(_) => "a module",
            ItemEnum::ExternCrate { .. } => "an external crate",
            ItemEnum::Use(_) => "an import",
            ItemEnum::Union(_) => "a union",
            ItemEnum::Struct(_) => "a struct",
            ItemEnum::StructField(_) => "a struct field",
            ItemEnum::Enum(_) => "an enum",
            ItemEnum::Variant(_) => "an enum variant",
            ItemEnum::Function(func) => {
                let mut func_kind = "a function";
                if let Some((param, _)) = func.sig.inputs.first()
                    && param == "self"
                {
                    func_kind = "a method";
                }

                func_kind
            }
            ItemEnum::Trait(_) => "a trait",
            ItemEnum::TraitAlias(_) => "a trait alias",
            ItemEnum::Impl(_) => "an impl block",
            ItemEnum::TypeAlias(_) => "a type alias",
            ItemEnum::Constant { .. } => "a constant",
            ItemEnum::Static(_) => "a static",
            ItemEnum::ExternType => "a foreign type",
            ItemEnum::Macro(_) => "a macro",
            ItemEnum::ProcMacro(_) => "a procedural macro",
            ItemEnum::Primitive(_) => "a primitive type",
            ItemEnum::AssocConst { .. } => "an associated constant",
            ItemEnum::AssocType { .. } => "an associated type",
        }
    }
}

fn get_external_crate_version(external_crate: &ExternalCrate) -> Option<Version> {
    if let Some(url) = &external_crate.html_root_url {
        url.trim_end_matches('/')
            .split('/')
            .next_back()
            .map(Version::parse)
            .and_then(|x| x.ok())
    } else {
        None
    }
}

/// Given a crate id for an external crate, return the corresponding [`PackageId`].
///
/// It panics if the provided crate id doesn't appear in the JSON documentation
/// for this crate—i.e. if it's not `0` or assigned to one of its transitive dependencies.
#[allow(clippy::disallowed_types)]
pub fn compute_package_id_for_crate_id(
    // The package id of the crate whose documentation we are currently processing.
    package_id: &PackageId,
    // The mapping from crate id to external crate object.
    external_crate_index: &FxHashMap<u32, ExternalCrate>,
    crate_id: u32,
    // There might be multiple crates in the dependency graph with the same name, causing
    // disambiguation issues.
    // To help out, you can specify `maybe_dependent`: the name of a crate that you think
    // depends on the crate you're trying to resolve.
    // This can narrow down the portion of the dependency graph that we need to search,
    // thus removing ambiguity.
    maybe_dependent_crate_name: Option<&str>,
    package_graph: &PackageGraph,
) -> Result<PackageId, anyhow::Error> {
    #[derive(Debug, Hash, Eq, PartialEq)]
    struct PackageLinkMetadata<'a> {
        id: &'a PackageId,
        name: &'a str,
        version: &'a Version,
    }

    /// Find a transitive dependency of `search_root` given its name (and maybe the version).
    /// It only returns `Some` if the dependency can be identified without ambiguity.
    fn find_transitive_dependency(
        package_graph: &PackageGraph,
        search_root: &PackageId,
        name: &str,
        version: Option<&Version>,
    ) -> Option<PackageId> {
        match _find_transitive_dependency(package_graph, search_root, name, version) {
            Ok(v) => v,
            Err(e) => {
                log_error!(
                    *e,
                    level: tracing::Level::WARN,
                    external_crate.name = %name,
                    external_crate.version = ?version,
                    search_root = %search_root.repr(),
                    "Failed to find transitive dependency"
                );
                None
            }
        }
    }

    fn _find_transitive_dependency(
        package_graph: &PackageGraph,
        search_root: &PackageId,
        name: &str,
        version: Option<&Version>,
    ) -> Result<Option<PackageId>, anyhow::Error> {
        let transitive_dependencies = package_graph
            .query_forward([search_root])
            .with_context(|| {
                format!(
                    "`{}` doesn't appear in the package graph for the current workspace",
                    search_root.repr()
                )
            })?
            .resolve();
        let expected_link_name = utils::normalize_crate_name(name);
        let package_candidates: IndexSet<_> = transitive_dependencies
            .links(guppy::graph::DependencyDirection::Forward)
            .filter(|link| utils::normalize_crate_name(link.to().name()) == expected_link_name)
            .map(|link| {
                let l = link.to();
                PackageLinkMetadata {
                    id: l.id(),
                    name: l.name(),
                    version: l.version(),
                }
            })
            .collect();
        if package_candidates.is_empty() {
            anyhow::bail!(
                "I could not find any crate named `{expected_link_name}` \
                among the dependencies of {search_root}",
            )
        }
        if package_candidates.len() == 1 {
            return Ok(Some(package_candidates.first().unwrap().id.to_owned()));
        }

        if let Some(expected_link_version) = version {
            let version_matcher = VersionMatcher::new(expected_link_version);
            let filtered_candidates: Vec<_> = package_candidates
                .iter()
                .filter(|l| version_matcher.matches(l.version))
                .collect();
            if filtered_candidates.is_empty() {
                let candidates = package_candidates
                    .iter()
                    .map(|l| format!("- {}@{}", l.name, l.version))
                    .collect::<Vec<_>>()
                    .join("\n");
                anyhow::bail!(
                    "Searching for `{expected_link_name}` among the transitive dependencies \
                    of `{search_root}` led to multiple results:\n{candidates}\n\
                    When the version ({expected_link_version}) was added to the search filters, \
                    no results come up. Could the inferred version be incorrect?\n\
                    This can happen if `{expected_link_name}` is using `#![doc(html_root_url = \"..\")]` \
                    with a URL that points to the documentation for a different (older?) version of itself."
                )
            }
            if filtered_candidates.len() == 1 {
                return Ok(Some(filtered_candidates.first().unwrap().id.to_owned()));
            }
        }

        Ok(None)
    }

    if crate_id == 0 {
        return Ok(package_id.clone());
    }

    let external_crate = external_crate_index.get(&crate_id).ok_or_else(|| {
        anyhow!(
            "There is no external crate associated with id `{}` in the JSON documentation for `{}`",
            crate_id,
            package_id.repr()
        )
    })?;
    if TOOLCHAIN_CRATES.contains(&external_crate.name.as_str()) {
        return Ok(PackageId::new(external_crate.name.clone()));
    }
    let external_crate_version = get_external_crate_version(external_crate);
    if let Some(id) = find_transitive_dependency(
        package_graph,
        package_id,
        &external_crate.name,
        external_crate_version.as_ref(),
    ) {
        return Ok(id);
    }

    // We have multiple packages with the same name.
    // We need to disambiguate among them.
    if let Some(maybe_dependent_crate_name) = maybe_dependent_crate_name {
        let intermediate_crates: Vec<_> = external_crate_index
            .values()
            .filter(|c| c.name == maybe_dependent_crate_name)
            .collect();
        if intermediate_crates.len() == 1 {
            let intermediate_crate = intermediate_crates.first().unwrap();
            let intermediate_crate_version = get_external_crate_version(intermediate_crate);
            if let Some(intermediate_package_id) = find_transitive_dependency(
                package_graph,
                package_id,
                &intermediate_crate.name,
                intermediate_crate_version.as_ref(),
            ) && let Some(id) = find_transitive_dependency(
                package_graph,
                &intermediate_package_id,
                &external_crate.name,
                external_crate_version.as_ref(),
            ) {
                return Ok(id);
            }
        }
    }

    Err(anyhow!(
        "There are multiple packages named `{}` among the dependencies of {}. \
            In order to disambiguate among them, I need to know their versions.\n\
            Unfortunately, I couldn't extract the expected version for `{}` from HTML root URL included in the \
            JSON documentation for `{}`.\n\
            This due to a limitation in `rustdoc` itself: follow https://github.com/rust-lang/compiler-team/issues/622 \
            to track progress on this issue.",
        external_crate.name,
        package_id.repr(),
        external_crate.name,
        package_id.repr()
    ))
}
