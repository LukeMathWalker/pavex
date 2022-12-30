use std::borrow::Cow;
use std::collections::{BTreeSet, HashMap};
use std::fmt;
use std::fmt::{Display, Formatter};

use anyhow::{anyhow, Context};
use elsa::FrozenMap;
use guppy::graph::PackageGraph;
use guppy::{PackageId, Version};
use indexmap::IndexSet;
use rustdoc_types::{ExternalCrate, Item, ItemEnum, ItemKind, Visibility};

use crate::language::ImportPath;
use crate::rustdoc::package_id_spec::PackageIdSpecification;
use crate::rustdoc::{compute::compute_crate_docs, utils, CannotGetCrateData, TOOLCHAIN_CRATES};

/// The main entrypoint for accessing the documentation of the crates
/// in a specific `PackageGraph`.
///
/// It takes care of:
/// - Computing and caching the JSON documentation for crates in the graph;
/// - Execute queries that span the documentation of multiple crates (e.g. following crate
///   re-exports or star re-exports).
pub struct CrateCollection(FrozenMap<PackageIdSpecification, Box<Crate>>, PackageGraph);

impl fmt::Debug for CrateCollection {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.1)
    }
}

impl CrateCollection {
    /// Initialise the collection for a `PackageGraph`.
    pub fn new(package_graph: PackageGraph) -> Self {
        Self(FrozenMap::new(), package_graph)
    }

    /// Compute the documentation for the crate associated with a specific [`PackageId`].
    ///
    /// It will be retrieved from [`CrateCollection`]'s internal cache if it was computed before.
    pub fn get_or_compute_crate_by_package_id(
        &self,
        package_id: &PackageId,
    ) -> Result<&Crate, CannotGetCrateData> {
        let package_spec = PackageIdSpecification::from_package_id(package_id, &self.1);
        if self.0.get(&package_spec).is_none() {
            let krate = compute_crate_docs(
                self.1.workspace().target_directory().as_std_path(),
                &package_spec,
            )?;
            let krate = Crate::new(self, krate, package_id.to_owned());
            self.0.insert(package_spec.clone(), Box::new(krate));
        }
        Ok(self.get_crate_by_package_id_spec(&package_spec))
    }

    /// Retrieve the documentation for the crate associated with [`PackageId`] from
    /// [`CrateCollection`]'s internal cache if it was computed before.
    ///
    /// It panics if no documentation is found for the specified [`PackageId`].
    pub fn get_crate_by_package_id(&self, package_id: &PackageId) -> &Crate {
        let package_spec = PackageIdSpecification::from_package_id(package_id, &self.1);
        self.get_crate_by_package_id_spec(&package_spec)
    }

    /// Retrieve the documentation for the crate associated with [`PackageIdSpecification`] from
    /// [`CrateCollection`]'s internal cache if it was computed before.
    ///
    /// It panics if no documentation is found for the specified [`PackageIdSpecification`].
    pub fn get_crate_by_package_id_spec(&self, package_spec: &PackageIdSpecification) -> &Crate {
        self.0.get(package_spec).unwrap_or_else(|| {
            panic!(
                "No JSON docs were found for the following package ID specification: {:?}",
                package_spec
            )
        })
    }

    /// Retrieve type information given its [`GlobalItemId`].
    ///
    /// It panics if no item is found for the specified [`GlobalItemId`].
    pub fn get_type_by_global_type_id(&self, type_id: &GlobalItemId) -> &Item {
        let krate = self.get_crate_by_package_id(&type_id.package_id);
        krate.get_type_by_local_type_id(&type_id.rustdoc_item_id)
    }

    /// Retrieve information about an item given its path and the id of the package where
    /// it was defined.
    pub fn get_item_by_resolved_path(
        &self,
        path: &[String],
        package_id: &PackageId,
    ) -> Result<Result<ResolvedItem<'_>, GetItemByResolvedPathError>, CannotGetCrateData> {
        let krate = self.get_or_compute_crate_by_package_id(package_id)?;
        if let Ok(type_id) = krate.get_type_id_by_path(&path) {
            let i = self.get_type_by_global_type_id(type_id);
            return Ok(Ok(ResolvedItem {
                item: Cow::Borrowed(i),
                item_id: type_id.to_owned(),
                parent: None,
            }));
        }
        // The path might be pointing to a method, which is not a type.
        // We drop the last segment to see if we can get a hit on the struct/enum type
        // to which the method belongs.
        if path.len() < 3 {
            // It has to be at least three segments - crate name, type name, method name.
            // If it's shorter than three, it's just an unknown path.
            return Ok(Err(UnknownItemPath {
                path: path.to_vec(),
            }
            .into()));
        }
        let (method_name, type_path_segments) = path.split_last().unwrap();

        if let Ok(parent_type_id) = krate.get_type_id_by_path(type_path_segments) {
            let parent = self.get_type_by_global_type_id(parent_type_id);
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
                                    return Ok(Ok(ResolvedItem {
                                        item: Cow::Borrowed(impl_item),
                                        item_id: GlobalItemId {
                                            package_id: krate.core.package_id.clone(),
                                            rustdoc_item_id: impl_item_id.to_owned(),
                                        },
                                        parent: Some(Cow::Borrowed(parent)),
                                    }));
                                }
                            }
                        }
                    }
                    ItemEnum::Function(_) => {
                        if child.name.as_ref() == Some(method_name) {
                            return Ok(Ok(ResolvedItem {
                                item: Cow::Borrowed(child),
                                item_id: GlobalItemId {
                                    package_id: krate.core.package_id.clone(),
                                    rustdoc_item_id: child_id.to_owned(),
                                },
                                parent: Some(Cow::Borrowed(parent)),
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
        let krate = self.get_crate_by_package_id(&type_id.package_id);
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
                used_by_krate.compute_package_id_for_crate_id(local_type_summary.crate_id, self),
                local_type_summary.path.clone(),
            )
        };
        let definition_krate = self.get_or_compute_crate_by_package_id(&definition_package_id)?;
        let type_id = definition_krate.get_type_id_by_path(&path)?;
        let canonical_path = self.get_canonical_path_by_global_type_id(type_id)?;
        Ok((type_id.clone(), canonical_path))
    }
}

/// The output of [`CrateCollection::find_item_by_resolved_path`].
///
/// If the path points to a "free-standing" item, `parent` is set to `None`.
/// Examples: a function, a struct, an enum.
///
/// If the item is "attached" to another parent item, `parent` is set to `Some`.
/// Examples: a trait method and the respective trait definition, a method and the struct it is
/// defined on, etc.
#[derive(Debug, Clone)]
pub struct ResolvedItem<'a> {
    pub item: Cow<'a, Item>,
    pub item_id: GlobalItemId,
    pub parent: Option<Cow<'a, Item>>,
}

/// Thin wrapper around [`rustdoc_types::Crate`] to:
/// - bundle a derived index (path <> id);
/// - provide query helpers with good error messages.
///
/// It also records the `PackageId` for the corresponding crate within the dependency tree
/// for the workspace it belongs to.
#[derive(Debug, Clone)]
pub struct Crate {
    pub(crate) core: CrateCore,
    /// An index to lookup the global id of a type given a local importable path
    /// that points at it.
    ///
    /// The index does NOT contain macros, since macros and types live in two
    /// different namespaces and can contain items with the same name.
    /// E.g. `core::clone::Clone` is both a trait and a derive macro.
    types_path_index: HashMap<Vec<String>, GlobalItemId>,
    public_local_path_index: HashMap<GlobalItemId, BTreeSet<Vec<String>>>,
}

#[derive(Debug, Clone)]
pub(crate) struct CrateCore {
    pub(crate) package_id: PackageId,
    krate: rustdoc_types::Crate,
}

impl CrateCore {
    /// Given a crate id, return the corresponding external crate object.
    /// We also try to return the crate version, if we manage to parse it out of the crate HTML
    /// root URL.
    fn get_external_crate_name(&self, crate_id: u32) -> Option<(&ExternalCrate, Option<Version>)> {
        self.krate.get_external_crate_name(crate_id)
    }

    /// Given a crate id, return the corresponding [`PackageId`].
    ///
    /// It panics if the provided crate id does not appear in the JSON documentation
    /// for this crate - i.e. if it's not `0` or assigned to one of its transitive dependencies.
    pub fn compute_package_id_for_crate_id(
        &self,
        crate_id: u32,
        collection: &CrateCollection,
    ) -> PackageId {
        #[derive(Debug, Hash, Eq, PartialEq)]
        struct PackageLinkMetadata<'a> {
            id: &'a PackageId,
            name: &'a str,
            version: &'a Version,
        }

        if crate_id == 0 {
            return self.package_id.clone();
        }

        let package_graph = &collection.1;
        let (external_crate, external_crate_version) =
            self.get_external_crate_name(crate_id)
                .ok_or_else(|| {
                    anyhow!(
                        "There is no external crate associated with id `{}` in the JSON documentation for `{}`",
                        crate_id,
                        self.package_id.repr()
                    )
                }).unwrap();
        if TOOLCHAIN_CRATES.contains(&external_crate.name.as_str()) {
            return PackageId::new(external_crate.name.clone());
        }

        let transitive_dependencies = package_graph
            .query_forward([&self.package_id])
            .with_context(|| {
                format!(
                    "`{}` does not appear in the package graph for the current workspace",
                    &self.package_id.repr()
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
                self.package_id
            ))
            .unwrap()
        }
        if package_candidates.len() == 1 {
            return package_candidates.first().unwrap().id.to_owned();
        }

        // We have multiple packages with the same name.
        // We try to use the version to identify the one we are looking for.
        // If we don't have a version, we panic: better than picking one randomly and failing
        // later with a confusing message.
        if let Some(expected_link_version) = external_crate_version.as_ref() {
            package_candidates
                .into_iter()
                .find(|l| l.version == expected_link_version)
                .ok_or_else(|| {
                    anyhow!(
                        "None of the dependencies of {} named `{}` matches the version we expect ({})",
                        self.package_id,
                        expected_link_name,
                        expected_link_version
                    )
                }).unwrap().id.to_owned()
        } else {
            Err(
                anyhow!(
                    "There are multiple packages named `{}` among the dependencies of {}. \
                    I was not able to extract the expected version for `{}` from the JSON documentation for {}, \
                    therefore I do not have a way to disambiguate among the matches we found",
                    expected_link_name,
                    self.package_id.repr(),
                    expected_link_name,
                    self.package_id.repr()
                )
            ).unwrap()
        }
    }
}

impl Crate {
    #[tracing::instrument(skip_all, name = "index_crate_docs", fields(package.id = package_id.repr()))]
    fn new(
        collection: &CrateCollection,
        krate: rustdoc_types::Crate,
        package_id: PackageId,
    ) -> Self {
        let crate_core = CrateCore { package_id, krate };
        let mut types_path_index: HashMap<_, _> = crate_core
            .krate
            .paths
            .iter()
            // We only want types, no macros
            .filter(|(_, summary)| !matches!(summary.kind, ItemKind::Macro | ItemKind::ProcDerive))
            .map(|(id, summary)| {
                (
                    summary.path.clone(),
                    GlobalItemId::new(id.to_owned(), crate_core.package_id.clone()),
                )
            })
            .collect();

        let mut public_local_path_index = HashMap::new();
        index_local_types(
            &crate_core,
            collection,
            vec![],
            &mut public_local_path_index,
            &crate_core.krate.root,
        );

        types_path_index.reserve(public_local_path_index.len());
        for (id, public_paths) in &public_local_path_index {
            for public_path in public_paths {
                if types_path_index.get(public_path).is_none() {
                    types_path_index.insert(public_path.to_owned(), id.to_owned());
                }
            }
        }

        Self {
            core: crate_core,
            types_path_index,
            public_local_path_index,
        }
    }

    /// Given a crate id, return the corresponding [`PackageId`].
    ///
    /// It panics if the provided crate id does not appear in the JSON documentation
    /// for this crate - i.e. if it's not `0` or assigned to one of its transitive dependencies.
    pub fn compute_package_id_for_crate_id(
        &self,
        crate_id: u32,
        collection: &CrateCollection,
    ) -> PackageId {
        self.core
            .compute_package_id_for_crate_id(crate_id, collection)
    }

    pub fn get_type_id_by_path(&self, path: &[String]) -> Result<&GlobalItemId, UnknownItemPath> {
        self.types_path_index
            .get(path)
            .ok_or_else(|| UnknownItemPath {
                path: path.to_owned(),
            })
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

    pub fn get_type_by_local_type_id(&self, id: &rustdoc_types::Id) -> &Item {
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
    /// This method returns a "canonical" importable path - i.e. the shortest importable path
    /// pointing at the type you specified.
    fn get_canonical_path(&self, type_id: &GlobalItemId) -> Result<&[String], anyhow::Error> {
        if let Some(path) = self.public_local_path_index.get(type_id) {
            Ok(path.iter().next().unwrap())
        } else {
            Err(anyhow::anyhow!(
                "Failed to find a publicly importable path for the type id `{:?}` in the index I computed for `{:?}`. \
                 This is likely to be a bug in pavex's handling of rustdoc's JSON output or in rustdoc itself.",
                type_id, self.core.package_id.repr()
            ))
        }
    }
}

fn index_local_types<'a>(
    crate_core: &'a CrateCore,
    collection: &'a CrateCollection,
    mut current_path: Vec<&'a str>,
    path_index: &mut HashMap<GlobalItemId, BTreeSet<Vec<String>>>,
    current_item_id: &rustdoc_types::Id,
) {
    // TODO: the way we handle `current_path` is extremely wasteful,
    //       we can likely reuse the same buffer throughout.
    let current_item = match crate_core.krate.index.get(current_item_id) {
        None => {
            if let Some(summary) = crate_core.krate.paths.get(current_item_id) {
                if summary.kind == ItemKind::Primitive {
                    // This is a known bug - see https://github.com/rust-lang/rust/issues/104064
                    return;
                }
            }
            panic!(
                "Failed to retrieve item id `{:?}` from the JSON `index` for package id `{}`.",
                &current_item_id,
                crate_core.package_id.repr()
            )
        }
        Some(i) => i,
    };

    // We do not want to index private items.
    if let Visibility::Default | Visibility::Crate | Visibility::Restricted { .. } =
        current_item.visibility
    {
        return;
    }

    match &current_item.inner {
        ItemEnum::Module(m) => {
            let current_path_segment = current_item
                .name
                .as_deref()
                .expect("All 'module' items have a 'name' property");
            current_path.push(current_path_segment);
            for item_id in &m.items {
                index_local_types(
                    crate_core,
                    collection,
                    current_path.clone(),
                    path_index,
                    item_id,
                );
            }
        }
        ItemEnum::Import(i) => {
            if let Some(imported_id) = &i.id {
                match crate_core.krate.index.get(imported_id) {
                    None => {
                        if let Some(imported_summary) = crate_core.krate.paths.get(imported_id) {
                            debug_assert!(imported_summary.crate_id != 0);
                            if let ItemKind::Module = imported_summary.kind {
                                // We are looking at a public re-export of another crate (e.g. `pub use hyper;`)
                                // or one of its modules.
                                // Due to how re-exports are handled in `rustdoc`, the re-exported
                                // items inside that foreign module will not be found in the `index`
                                // for this crate.
                                // We intentionally add foreign items to the index to get a "complete"
                                // picture of all the types available in this crate.
                                let external_crate_id = imported_summary.crate_id;
                                let external_package_id = crate_core
                                    .compute_package_id_for_crate_id(external_crate_id, collection);
                                // TODO: remove unwraps
                                let external_crate = collection
                                    .get_or_compute_crate_by_package_id(&external_package_id)
                                    .unwrap();
                                // It looks like we might fail to find the module item if there are
                                // no public types inside it.
                                // Example of this issue: `core::fmt::rt`.
                                if let Ok(foreign_item_id) =
                                    external_crate.get_type_id_by_path(&imported_summary.path)
                                {
                                    let foreign_item_id = foreign_item_id.rustdoc_item_id.clone();
                                    index_local_types(
                                        &external_crate.core,
                                        collection,
                                        current_path,
                                        path_index,
                                        &foreign_item_id,
                                    );
                                }
                            }
                        } else {
                            // TODO: this is firing for std's JSON docs. File a bug report.
                            // panic!("The imported id ({}) is not listed in the index nor in the path section of rustdoc's JSON output", imported_id.0)
                        }
                    }
                    Some(imported_item) => {
                        if let ItemEnum::Module(_) = imported_item.inner {
                            if !i.glob {
                                current_path.push(&i.name);
                            }
                        }
                        index_local_types(
                            crate_core,
                            collection,
                            current_path.clone(),
                            path_index,
                            imported_id,
                        );
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
            path_index
                .entry(GlobalItemId::new(
                    current_item_id.to_owned(),
                    crate_core.package_id.to_owned(),
                ))
                .or_default()
                .insert(path);
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
    pub path: ImportPath,
    pub kind: String,
}

impl Display for UnsupportedItemKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
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
    pub path: ImportPath,
}

impl Display for UnknownItemPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
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
    /// via `#[doc(html_root_url)]` - e.g. pointing at an outdated version of their docs (see
    /// https://github.com/tokio-rs/tracing/pull/2384 as an example).
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
