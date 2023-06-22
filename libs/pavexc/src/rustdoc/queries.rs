use std::borrow::Cow;
use std::collections::BTreeSet;
use std::sync::Arc;
use std::thread;

use ahash::{HashMap, HashMapExt};
use anyhow::{anyhow, Context};
use elsa::FrozenMap;
use guppy::graph::PackageGraph;
use guppy::{PackageId, Version};
use indexmap::IndexSet;
use rustdoc_types::{ExternalCrate, Item, ItemEnum, ItemKind, ItemSummary, Visibility};
use tracing::Span;

use crate::rustdoc::{compute::compute_crate_docs, utils, CannotGetCrateData, TOOLCHAIN_CRATES};
use crate::rustdoc::{ALLOC_PACKAGE_ID, CORE_PACKAGE_ID, STD_PACKAGE_ID};

use super::compute::{batch_compute_crate_docs, RustdocCacheKey, RustdocGlobalFsCache};

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
}

impl std::fmt::Debug for CrateCollection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CrateCollection")
            .field("package_graph", &self.package_graph)
            .field("disk_cache", &self.disk_cache)
            .finish()
    }
}

#[tracing::instrument]
fn compute_package_graph() -> Result<PackageGraph, anyhow::Error> {
    // `cargo metadata` seems to be the only reliable way of retrieving the path to
    // the root manifest of the current workspace for a Rust project.
    guppy::MetadataCommand::new()
        .exec()
        .map_err(|e| anyhow!(e))?
        .build_graph()
        .map_err(|e| anyhow!(e))
}

impl CrateCollection {
    /// Initialise the collection for a `PackageGraph`.
    pub fn new(project_fingerprint: String) -> Result<Self, anyhow::Error> {
        let span = Span::current();
        let thread_handle = thread::spawn(move || {
            let _guard = span.enter();
            compute_package_graph()
        });
        let cache = RustdocGlobalFsCache::new()?;

        let package_graph = thread_handle
            .join()
            .map_err(|_| anyhow!("The thread computing the package graph panicked"))?
            .context("Failed to compute the package graph for the current workspace")?;

        Ok(Self {
            package_id2krate: FrozenMap::new(),
            package_graph,
            disk_cache: cache,
            access_log: FrozenMap::new(),
            project_fingerprint,
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
                tracing::warn!(
                    error.msg = tracing::field::display(&e),
                    error.error_chain = tracing::field::debug(&e),
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
            package_graph: PackageGraph,
            cache: RustdocGlobalFsCache,
        ) -> (PackageId, Option<Crate>) {
            let cache_key = RustdocCacheKey::new(&package_id, &package_graph);
            match cache.get(&cache_key) {
                Ok(o) => (package_id, o),
                Err(e) => {
                    tracing::warn!(
                        error.msg = tracing::field::display(&e),
                        error.error_chain = tracing::field::debug(&e),
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
        let package_graph = self.package_graph.clone();
        let cache = self.disk_cache.clone();
        let tracing_span = Span::current().clone();
        let map_op = move |id| {
            tracing_span.in_scope(|| get_if_cached(id, package_graph.clone(), cache.clone()))
        };

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
        let results = batch_compute_crate_docs(&self.package_graph, to_be_computed.into_iter())?;

        for (package_id, krate) in results {
            let krate =
                Crate::new(self, krate, package_id.to_owned()).map_err(|e| CannotGetCrateData {
                    package_spec: package_id.to_string(),
                    source: Arc::new(e),
                })?;

            // Let's make sure to store them in the on-disk cache for next time.
            let cache_key = RustdocCacheKey::new(&package_id, &self.package_graph);
            if let Err(e) = self.disk_cache.insert(&cache_key, &krate) {
                tracing::warn!(
                    error.msg = tracing::field::display(&e),
                    error.error_chain = tracing::field::debug(&e),
                    package_id = package_id.repr(),
                    "Failed to store the computed JSON docs in the on-disk cache",
                );
            }
            self.package_id2krate
                .insert(package_id.to_owned(), Box::new(krate));
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
        match self.disk_cache.get(&cache_key) {
            Ok(Some(krate)) => {
                self.package_id2krate
                    .insert(package_id.to_owned(), Box::new(krate));
                return Ok(self.get_crate_by_package_id(package_id).unwrap());
            }
            Err(e) => {
                tracing::warn!(
                    error.msg = tracing::field::display(&e),
                    error.error_chain = tracing::field::debug(&e),
                    package_id = package_id.repr(),
                    "Failed to retrieve the documentation from the on-disk cache",
                );
            }
            Ok(None) => {}
        }

        // If we don't have them in the on-disk cache, we need to compute them.
        let krate = compute_crate_docs(&self.package_graph, &package_id)?;
        let krate =
            Crate::new(self, krate, package_id.to_owned()).map_err(|e| CannotGetCrateData {
                package_spec: package_id.to_string(),
                source: Arc::new(e),
            })?;

        // Let's make sure to store them in the on-disk cache for next time.
        if let Err(e) = self.disk_cache.insert(&cache_key, &krate) {
            tracing::warn!(
                error.msg = tracing::field::display(&e),
                error.error_chain = tracing::field::debug(&e),
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

    /// Retrieve type information given its [`GlobalItemId`].
    ///
    /// It panics if no item is found for the specified [`GlobalItemId`].
    pub fn get_type_by_global_type_id(&self, type_id: &GlobalItemId) -> Cow<'_, Item> {
        // Safe to unwrap since the package id is coming from a GlobalItemId.
        let krate = self.get_crate_by_package_id(&type_id.package_id).unwrap();
        krate.get_type_by_local_type_id(&type_id.rustdoc_item_id)
    }

    /// Retrieve information about an item given its path and the id of the package where
    /// it was defined.
    pub fn get_item_by_resolved_path(
        &self,
        path: &[String],
        package_id: &PackageId,
    ) -> Result<Result<ResolvedItemWithParent<'_>, GetItemByResolvedPathError>, CannotGetCrateData>
    {
        let krate = self.get_or_compute_crate_by_package_id(package_id)?;
        if let Ok(type_id) = krate.get_type_id_by_path(path, self)? {
            let i = self.get_type_by_global_type_id(&type_id);
            return Ok(Ok(ResolvedItemWithParent {
                item: ResolvedItem {
                    item: i,
                    item_id: type_id.to_owned(),
                },
                parent: None,
            }));
        }
        // The path might be pointing to a method, which is not a type.
        // We drop the last segment to see if we can get a hit on the struct/enum type
        // to which the method belongs.
        if path.len() < 3 {
            // It has to be at least three segments—crate name, type name, method name.
            // If it's shorter than three, it's just an unknown path.
            return Ok(Err(UnknownItemPath {
                path: path.to_vec(),
            }
            .into()));
        }
        let (method_name, type_path_segments) = path.split_last().unwrap();

        if let Ok(mut parent_type_id) = krate.get_type_id_by_path(type_path_segments, self)? {
            let mut parent = self.get_type_by_global_type_id(&parent_type_id);
            // The parent trait/struct might have been a re-export, so we need to make sure that we
            // are looking at the crate where it was originally defined when we start
            // following the local type ids that are encoded in the parent.
            let mut krate = self.get_or_compute_crate_by_package_id(&parent_type_id.package_id)?;

            // We eagerly check if the parent item is an alias, and if so we follow it
            // to the original type.
            // This might take multiple iterations, since the alias might point to another
            // alias.
            loop {
                let ItemEnum::Typedef(typedef) = &parent.inner else { break; };
                let rustdoc_types::Type::ResolvedPath(p) = &typedef.type_ else { break; };
                // The aliased type might be a re-export of a foreign type,
                // therefore we go through the summary here rather than
                // going straight for a local id lookup.
                let summary = krate.get_type_summary_by_local_type_id(&p.id).unwrap();
                let source_package_id = krate
                    .compute_package_id_for_crate_id(summary.crate_id, &self)
                    .map_err(|e| CannotGetCrateData {
                        package_spec: summary.crate_id.to_string(),
                        source: Arc::new(e),
                    })?;
                krate = self.get_or_compute_crate_by_package_id(&source_package_id)?;
                if let Ok(type_id) = krate.get_type_id_by_path(&summary.path, &self)? {
                    parent_type_id = type_id;
                } else {
                    return Ok(Err(UnknownItemPath {
                        path: summary.path.clone(),
                    }
                    .into()));
                }
                parent = self.get_type_by_global_type_id(&parent_type_id);
            }

            let children_ids = match &parent.inner {
                ItemEnum::Struct(s) => &s.impls,
                ItemEnum::Enum(enum_) => &enum_.impls,
                ItemEnum::Trait(trait_) => &trait_.items,
                item => {
                    return Ok(Err(UnsupportedItemKind {
                        path: path.to_owned(),
                        kind: item.kind().to_owned(),
                    }
                    .into()));
                }
            };
            for child_id in children_ids {
                let child = krate.get_type_by_local_type_id(child_id);
                match &child.inner {
                    ItemEnum::Impl(impl_block) => {
                        // We are completely ignoring the bounds attached to the implementation block.
                        // This can lead to issues: the same method can be defined multiple
                        // times in different implementation blocks with non-overlapping constraints.
                        for impl_item_id in &impl_block.items {
                            let impl_item = krate.get_type_by_local_type_id(impl_item_id);
                            if impl_item.name.as_ref() == Some(method_name) {
                                if let ItemEnum::Function(_) = &impl_item.inner {
                                    return Ok(Ok(ResolvedItemWithParent {
                                        item: ResolvedItem {
                                            item: impl_item,
                                            item_id: GlobalItemId {
                                                package_id: krate.core.package_id.clone(),
                                                rustdoc_item_id: impl_item_id.to_owned(),
                                            },
                                        },
                                        parent: Some(ResolvedItem {
                                            item: parent,
                                            item_id: parent_type_id.to_owned(),
                                        }),
                                    }));
                                }
                            }
                        }
                    }
                    ItemEnum::Function(_) => {
                        if child.name.as_ref() == Some(method_name) {
                            return Ok(Ok(ResolvedItemWithParent {
                                item: ResolvedItem {
                                    item: child,
                                    item_id: GlobalItemId {
                                        package_id: krate.core.package_id.clone(),
                                        rustdoc_item_id: child_id.to_owned(),
                                    },
                                },
                                parent: Some(ResolvedItem {
                                    item: parent,
                                    item_id: parent_type_id.to_owned(),
                                }),
                            }));
                        }
                    }
                    i => {
                        dbg!(i);
                        unreachable!()
                    }
                }
            }
        }
        Ok(Err(UnknownItemPath {
            path: path.to_owned(),
        }
        .into()))
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
    ) -> Result<(GlobalItemId, &[String]), anyhow::Error> {
        let (definition_package_id, path) = {
            let used_by_krate = self.get_or_compute_crate_by_package_id(used_by_package_id)?;
            let local_type_summary = used_by_krate.get_type_summary_by_local_type_id(item_id)?;
            (
                used_by_krate.compute_package_id_for_crate_id(local_type_summary.crate_id, self)?,
                local_type_summary.path.clone(),
            )
        };
        let definition_krate = self.get_or_compute_crate_by_package_id(&definition_package_id)?;
        let type_id = definition_krate.get_type_id_by_path(&path, self)??;
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
            tracing::warn!(
                error.msg = tracing::field::display(&e),
                error.error_chain = tracing::field::debug(&e),
                "Failed to persist the crate access log to the on-disk cache",
            );
        }
    }
}

/// The output of [`CrateCollection::get_item_by_resolved_path`].
///
/// If the path points to a "free-standing" item, `parent` is set to `None`.
/// Examples: a function, a struct, an enum.
///
/// If the item is "attached" to another parent item, `parent` is set to `Some`.
/// Examples: a trait method and the respective trait definition, a method and the struct it is
/// defined on, etc.
#[derive(Debug, Clone)]
pub struct ResolvedItemWithParent<'a> {
    pub item: ResolvedItem<'a>,
    pub parent: Option<ResolvedItem<'a>>,
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
    pub(super) import_path2id: HashMap<Vec<String>, rustdoc_types::Id>,
    /// A mapping that keeps track of re-exports of types (or modules!) from
    /// other crates.
    ///
    /// The key is the path under which the type is re-exported.
    /// The value is a tuple containing:
    /// - the path of the type in the original crate;
    /// - the id of the original crate in the `external_crates` section of the JSON documentation.
    ///
    /// E.g. `pub use hyper::server as sx;` in `lib.rs` would have an entry in this map
    /// with key `["my_crate", "sx"]` and value `(["hyper", "server"], _)`.
    pub(super) re_exports: HashMap<Vec<String>, (Vec<String>, u32)>,
    /// A mapping from a type id to all the public paths under which it can be imported
    /// from another crate.
    pub(super) id2public_import_paths: HashMap<rustdoc_types::Id, BTreeSet<Vec<String>>>,
    /// A mapping from a type id to all the non-public paths under which it can be imported
    /// from within the same crate.
    pub(super) id2private_import_paths: HashMap<rustdoc_types::Id, BTreeSet<Vec<String>>>,
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
    pub external_crates: std::collections::HashMap<u32, ExternalCrate>,
    /// A mapping from the id of a type to its fully qualified path.
    /// Primarily useful for foreign items that are being re-exported by this crate.
    pub paths: std::collections::HashMap<rustdoc_types::Id, ItemSummary>,
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
                let (start, end) = index.item_id2delimiters.get(id)?;
                let bytes = index.items[*start..*end].to_vec();
                let item = serde_json::from_slice(&bytes).expect(
                    "Failed to deserialize an item from a lazy `rustdoc` index. This is a bug.",
                );
                Some(Cow::Owned(item))
            }
        }
    }
}

#[derive(Debug, Clone)]
/// See [`CrateItemIndex`] for more information.
pub(crate) struct EagerCrateItemIndex {
    pub index: std::collections::HashMap<rustdoc_types::Id, Item>,
}

#[derive(Debug, Clone)]
/// See [`CrateItemIndex`] for more information.
pub(crate) struct LazyCrateItemIndex {
    pub(super) items: Vec<u8>,
    pub(super) item_id2delimiters: HashMap<rustdoc_types::Id, (usize, usize)>,
}

impl CrateCore {
    /// Given a crate id, return the corresponding [`PackageId`].
    ///
    /// It panics if the provided crate id doesn't appear in the JSON documentation
    /// for this crate—i.e. if it's not `0` or assigned to one of its transitive dependencies.
    pub fn compute_package_id_for_crate_id(
        &self,
        crate_id: u32,
        collection: &CrateCollection,
    ) -> Result<PackageId, anyhow::Error> {
        compute_package_id_for_crate_id(
            &self.package_id,
            &self.krate.external_crates,
            crate_id,
            &collection.package_graph,
        )
    }
}

impl Crate {
    #[tracing::instrument(skip_all, name = "index_crate_docs", fields(package.id = package_id.repr()))]
    fn new(
        collection: &CrateCollection,
        krate: rustdoc_types::Crate,
        package_id: PackageId,
    ) -> Result<Self, anyhow::Error> {
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

        let mut id2public_import_paths = HashMap::new();
        let mut id2private_import_paths = HashMap::new();
        let mut re_exports = HashMap::new();
        index_local_types(
            &krate,
            &package_id,
            collection,
            vec![],
            &mut id2public_import_paths,
            &mut id2private_import_paths,
            &mut re_exports,
            &krate.root,
        )?;

        import_path2id.reserve(id2public_import_paths.len());
        for (id, paths) in id2public_import_paths
            .iter()
            .chain(&id2private_import_paths)
        {
            for path in paths {
                if import_path2id.get(path).is_none() {
                    import_path2id.insert(path.to_owned(), id.to_owned());
                }
            }
        }

        Ok(Self {
            core: CrateCore {
                package_id,
                krate: CrateData {
                    root_item_id: krate.root,
                    index: CrateItemIndex::Eager(EagerCrateItemIndex { index: krate.index }),
                    external_crates: krate.external_crates,
                    format_version: krate.format_version,
                    paths: krate.paths,
                },
            },
            import_path2id,
            id2public_import_paths,
            id2private_import_paths,
            re_exports,
        })
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
        self.core
            .compute_package_id_for_crate_id(crate_id, collection)
    }

    pub fn get_type_id_by_path(
        &self,
        path: &[String],
        krate_collection: &CrateCollection,
    ) -> Result<Result<GlobalItemId, UnknownItemPath>, CannotGetCrateData> {
        if let Some(id) = self.import_path2id.get(path) {
            return Ok(Ok(GlobalItemId::new(
                id.to_owned(),
                self.core.package_id.to_owned(),
            )));
        }

        for (re_exported_path_prefix, (source_path_prefix, external_crate_num)) in &self.re_exports
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
                    .compute_package_id_for_crate_id(*external_crate_num, krate_collection)
                    .unwrap();
                let source_krate = krate_collection
                    .get_or_compute_crate_by_package_id(&source_package_id)
                    .unwrap();
                if let Ok(source_id) =
                    source_krate.get_type_id_by_path(&original_source_path, krate_collection)
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
    fn get_type_summary_by_local_type_id(
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

    pub fn get_type_by_local_type_id(&self, id: &rustdoc_types::Id) -> Cow<'_, Item> {
        let type_ = self.core.krate.index.get(id);
        if type_.is_none() {
            panic!(
                "Failed to look up the type id `{}` in the rustdoc's index for package `{}`.",
                id.0,
                self.core.package_id.repr()
            )
        }
        type_.unwrap()
    }

    /// Types can be exposed under multiple paths.
    /// This method returns a "canonical" importable path—i.e. the shortest importable path
    /// pointing at the type you specified.
    fn get_canonical_path(&self, type_id: &GlobalItemId) -> Result<&[String], anyhow::Error> {
        if type_id.package_id == self.core.package_id {
            if let Some(paths) = self.id2public_import_paths.get(&type_id.rustdoc_item_id) {
                return Ok(paths.first().unwrap());
            }
            if let Some(paths) = self.id2private_import_paths.get(&type_id.rustdoc_item_id) {
                return Ok(paths.first().unwrap());
            }
        }
        Err(anyhow::anyhow!(
            "Failed to find an importable path for the type id `{:?}` in the index I computed for `{:?}`. \
            This is likely to be a bug in pavex's handling of rustdoc's JSON output or in rustdoc itself.",
            type_id, self.core.package_id.repr()
        ))
    }
}

fn index_local_types<'a>(
    krate: &'a rustdoc_types::Crate,
    package_id: &'a PackageId,
    collection: &'a CrateCollection,
    mut current_path: Vec<&'a str>,
    public_path_index: &mut HashMap<rustdoc_types::Id, BTreeSet<Vec<String>>>,
    private_path_index: &mut HashMap<rustdoc_types::Id, BTreeSet<Vec<String>>>,
    re_exports: &mut HashMap<Vec<String>, (Vec<String>, u32)>,
    current_item_id: &rustdoc_types::Id,
) -> Result<(), anyhow::Error> {
    // TODO: the way we handle `current_path` is extremely wasteful,
    //       we can likely reuse the same buffer throughout.
    let current_item = match krate.index.get(current_item_id) {
        None => {
            if let Some(summary) = krate.paths.get(current_item_id) {
                if summary.kind == ItemKind::Primitive {
                    // This is a known bug—see https://github.com/rust-lang/rust/issues/104064
                    return Ok(());
                }
            }
            panic!(
                "Failed to retrieve item id `{:?}` from the JSON `index` for package id `{}`.",
                &current_item_id,
                package_id.repr()
            )
        }
        Some(i) => i,
    };

    match &current_item.inner {
        ItemEnum::Module(m) => {
            let current_path_segment = current_item
                .name
                .as_deref()
                .expect("All 'module' items have a 'name' property");
            current_path.push(current_path_segment);
            for item_id in &m.items {
                index_local_types(
                    krate,
                    package_id,
                    collection,
                    current_path.clone(),
                    public_path_index,
                    private_path_index,
                    re_exports,
                    item_id,
                )?;
            }
        }
        ItemEnum::Import(i) => {
            if let Some(imported_id) = &i.id {
                match krate.index.get(imported_id) {
                    None => {
                        if let Some(imported_summary) = krate.paths.get(imported_id) {
                            debug_assert!(imported_summary.crate_id != 0);
                            // We are looking at a public re-export of another crate
                            // (e.g. `pub use hyper;`), one of its modules or one of its items.
                            // Due to how re-exports are handled in `rustdoc`, the re-exported
                            // items inside that foreign module will not be found in the `index`
                            // for this crate.
                            // We intentionally add foreign items to the index to get a "complete"
                            // picture of all the types available in this crate.
                            let external_crate_id = imported_summary.crate_id;
                            let re_exported_path = &imported_summary.path;
                            current_path.push(&i.name);

                            re_exports.insert(
                                current_path.into_iter().map(|s| s.to_string()).collect(),
                                (re_exported_path.to_owned(), external_crate_id),
                            );
                        } else {
                            // TODO: this is firing for std's JSON docs. File a bug report.
                            // panic!("The imported id ({}) is not listed in the index nor in the path section of rustdoc's JSON output", imported_id.0)
                        }
                    }
                    Some(imported_item) => {
                        if let ItemEnum::Module(re_exported_module) = &imported_item.inner {
                            if !i.glob {
                                current_path.push(&i.name);
                            }
                            for re_exported_item_id in &re_exported_module.items {
                                index_local_types(
                                    krate,
                                    package_id,
                                    collection,
                                    current_path.clone(),
                                    public_path_index,
                                    private_path_index,
                                    re_exports,
                                    re_exported_item_id,
                                )?;
                            }
                        } else {
                            index_local_types(
                                krate,
                                package_id,
                                collection,
                                current_path.clone(),
                                public_path_index,
                                private_path_index,
                                re_exports,
                                imported_id,
                            )?;
                        }
                    }
                }
            }
        }
        ItemEnum::Trait(_)
        | ItemEnum::Function(_)
        | ItemEnum::Enum(_)
        | ItemEnum::Struct(_)
        | ItemEnum::Typedef(_) => {
            let name = current_item.name.as_deref().expect(
                "All 'struct', 'function', 'enum', 'typedef' and 'trait' items have a 'name' property",
            );
            current_path.push(name);
            let path = current_path.into_iter().map(|s| s.to_string()).collect();

            let index = if current_item.visibility == Visibility::Public {
                public_path_index
            } else {
                private_path_index
            };
            index
                .entry(current_item_id.to_owned())
                .or_default()
                .insert(path);
        }
        _ => {}
    }
    Ok(())
}

/// An identifier that unequivocally points to a type within a [`CrateCollection`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GlobalItemId {
    pub(crate) rustdoc_item_id: rustdoc_types::Id,
    pub(crate) package_id: PackageId,
}

impl GlobalItemId {
    fn new(raw_id: rustdoc_types::Id, package_id: PackageId) -> Self {
        Self {
            rustdoc_item_id: raw_id,
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
            "I could not find '{path}' in the auto-generated documentation for '{krate}'"
        )
    }
}

trait RustdocCrateExt {
    /// Given a crate id, return the corresponding external crate object.
    /// We try to guess the crate version by parsing it out of the root URL for the HTML documentation.
    /// The extracted version is not guaranteed to be correct: crates can set an arbitrary root URL
    /// via `#[doc(html_root_url)]`—e.g. pointing at an outdated version of their docs (see
    /// <https://github.com/tokio-rs/tracing/pull/2384> as an example).
    fn get_external_crate_name(&self, crate_id: u32) -> Option<(&ExternalCrate, Option<Version>)>;
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
            ItemEnum::Import(_) => "an import",
            ItemEnum::Union(_) => "a union",
            ItemEnum::Struct(_) => "a struct",
            ItemEnum::StructField(_) => "a struct field",
            ItemEnum::Enum(_) => "an enum",
            ItemEnum::Variant(_) => "an enum variant",
            // TODO: this could also be a method! How do we find out?
            ItemEnum::Function(_) => "a function",
            ItemEnum::Trait(_) => "a trait",
            ItemEnum::TraitAlias(_) => "a trait alias",
            ItemEnum::Impl(_) => "an impl block",
            ItemEnum::Typedef(_) => "a type definition",
            ItemEnum::OpaqueTy(_) => "an opaque type",
            ItemEnum::Constant(_) => "a constant",
            ItemEnum::Static(_) => "a static",
            ItemEnum::ForeignType => "a foreign type",
            ItemEnum::Macro(_) => "a macro",
            ItemEnum::ProcMacro(_) => "a procedural macro",
            ItemEnum::Primitive(_) => "a primitive type",
            ItemEnum::AssocConst { .. } => "an associated constant",
            ItemEnum::AssocType { .. } => "an associated type",
        }
    }
}

impl RustdocCrateExt for rustdoc_types::Crate {
    fn get_external_crate_name(&self, crate_id: u32) -> Option<(&ExternalCrate, Option<Version>)> {
        let external_crate = self.external_crates.get(&crate_id);
        if let Some(external_crate) = external_crate {
            let version = if let Some(url) = &external_crate.html_root_url {
                url.trim_end_matches('/')
                    .split('/')
                    .last()
                    .map(Version::parse)
                    .and_then(|x| x.ok())
            } else {
                None
            };
            Some((external_crate, version))
        } else {
            None
        }
    }
}

fn get_external_crate_name(
    external_crates: &std::collections::HashMap<u32, ExternalCrate>,
    crate_id: u32,
) -> Option<(&ExternalCrate, Option<Version>)> {
    let external_crate = external_crates.get(&crate_id);
    if let Some(external_crate) = external_crate {
        let version = if let Some(url) = &external_crate.html_root_url {
            url.trim_end_matches('/')
                .split('/')
                .last()
                .map(Version::parse)
                .and_then(|x| x.ok())
        } else {
            None
        };
        Some((external_crate, version))
    } else {
        None
    }
}

/// Given a crate id for an external crate, return the corresponding [`PackageId`].
///
/// It panics if the provided crate id doesn't appear in the JSON documentation
/// for this crate—i.e. if it's not `0` or assigned to one of its transitive dependencies.
pub fn compute_package_id_for_crate_id(
    // The package id of the crate whose documentation we are currently processing.
    package_id: &PackageId,
    // The mapping from crate id to external crate object.
    external_crate_index: &std::collections::HashMap<u32, ExternalCrate>,
    crate_id: u32,
    package_graph: &PackageGraph,
) -> Result<PackageId, anyhow::Error> {
    #[derive(Debug, Hash, Eq, PartialEq)]
    struct PackageLinkMetadata<'a> {
        id: &'a PackageId,
        name: &'a str,
        version: &'a Version,
    }

    if crate_id == 0 {
        return Ok(package_id.clone());
    }

    let (external_crate, external_crate_version) =
        get_external_crate_name(external_crate_index, crate_id)
            .ok_or_else(|| {
                anyhow!(
            "There is no external crate associated with id `{}` in the JSON documentation for `{}`",
            crate_id,
            package_id.repr()
        )
            })
            .unwrap();
    if TOOLCHAIN_CRATES.contains(&external_crate.name.as_str()) {
        return Ok(PackageId::new(external_crate.name.clone()));
    }

    let transitive_dependencies = package_graph
        .query_forward([package_id])
        .with_context(|| {
            format!(
                "`{}` doesn't appear in the package graph for the current workspace",
                package_id.repr()
            )
        })
        .unwrap()
        .resolve();
    let expected_link_name = utils::normalize_crate_name(&external_crate.name);
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
        Err(anyhow!(
            "I could not find any crate named `{}` among the dependencies of {}",
            expected_link_name,
            package_id
        ))
        .unwrap()
    }
    if package_candidates.len() == 1 {
        return Ok(package_candidates.first().unwrap().id.to_owned());
    }

    // We have multiple packages with the same name.
    // We try to use the version to identify the one we are looking for.
    // If we don't have a version, we panic: better than picking one randomly and failing
    // later with a confusing message.
    if let Some(expected_link_version) = external_crate_version.as_ref() {
        Ok(package_candidates
            .into_iter()
            .find(|l| l.version == expected_link_version)
            .ok_or_else(|| {
                anyhow!(
                    "None of the dependencies of {} named `{}` matches the version we expect ({})",
                    package_id,
                    expected_link_name,
                    expected_link_version
                )
            })?
            .id
            .to_owned())
    } else {
        Err(
            anyhow!(
                "There are multiple packages named `{}` among the dependencies of {}. \
                In order to disambiguate among them, I need to know their versions.\n\
                Unfortunately, I couldn't extract the expected version for `{}` from HTML root URL included in the \
                JSON documentation for `{}`.\n\
                This due to a limitation in `rustdoc` itself: follow https://github.com/rust-lang/compiler-team/issues/622 \
                to track progress on this issue.",
                expected_link_name,
                package_id.repr(),
                expected_link_name,
                package_id.repr()
            )
        )
    }
}
