use std::collections::{BTreeSet, HashMap};
use std::fmt::{Display, Formatter};

use anyhow::{anyhow, Context};
use guppy::graph::PackageGraph;
use guppy::{PackageId, Version};
use rustdoc_types::{ExternalCrate, Item, ItemEnum, ItemKind, Visibility};

use crate::language::ImportPath;
use crate::rustdoc::package_id_spec::PackageIdSpecification;
use crate::rustdoc::{compute::get_crate_data, CannotGetCrateData, TOOLCHAIN_CRATES};

#[derive(Debug, Clone)]
/// The main entrypoint for accessing the documentation of the crates
/// in a specific `PackageGraph`.
///
/// It takes care of:
/// - Computing and caching the JSON documentation for crates in the graph;
/// - Execute queries that span the documentation of multiple crates (e.g. following crate
///   re-exports or star re-exports).
pub struct CrateCollection(HashMap<PackageIdSpecification, Crate>, PackageGraph);

impl CrateCollection {
    /// Initialise the collection for a `PackageGraph`.
    pub fn new(package_graph: PackageGraph) -> Self {
        Self(Default::default(), package_graph)
    }

    /// Compute the documentation for the crate associated with a specific [`PackageId`].
    ///
    /// It will be retrieved from [`CrateCollection`]'s internal cache if it was computed before.
    pub fn get_or_compute_crate_by_package_id(
        &mut self,
        package_id: &PackageId,
    ) -> Result<&Crate, CannotGetCrateData> {
        let package_spec = PackageIdSpecification::from_package_id(package_id, &self.1);
        if self.0.get(&package_spec).is_none() {
            let krate = get_crate_data(
                self.1.workspace().target_directory().as_std_path(),
                &package_spec,
            )?;
            let krate = Crate::new(self, krate, package_id.to_owned());
            self.0.insert(package_spec.clone(), krate);
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
        &self.0.get(package_spec).unwrap_or_else(|| {
            panic!(
                "No JSON docs were found for the following package ID specification: {:?}",
                package_spec
            )
        })
    }

    /// Retrieve type information given its [`GlobalTypeId`].
    ///
    /// It panics if no item is found for the specified [`GlobalTypeId`].
    pub fn get_type_by_global_type_id(&self, type_id: &GlobalTypeId) -> &Item {
        let krate = self.get_crate_by_package_id(&type_id.package_id);
        krate.get_type_by_local_type_id(&type_id.raw_id)
    }

    /// Retrieve the canonical path for a struct, enum or function given its [`GlobalTypeId`].
    ///
    /// It panics if no item is found for the specified [`GlobalTypeId`].
    pub fn get_canonical_path_by_global_type_id(&self, type_id: &GlobalTypeId) -> &[String] {
        let krate = self.get_crate_by_package_id(&type_id.package_id);
        krate.get_canonical_path(type_id)
    }

    /// Retrieve the package id of the crate where an item (identified by its **local** id)
    /// was originally defined.
    pub fn get_defining_package_id_by_local_type_id(
        &mut self,
        used_by_package_id: &PackageId,
        item_id: &rustdoc_types::Id,
    ) -> Result<PackageId, anyhow::Error> {
        self.get_or_compute_crate_by_package_id(used_by_package_id)?;
        let used_by_krate = self.get_crate_by_package_id(used_by_package_id);
        let crate_id = used_by_krate
            .get_type_summary_by_local_type_id(item_id)?
            .crate_id;
        let type_package_id = used_by_krate.compute_package_id_for_crate_id(crate_id, &self);
        Ok(type_package_id)
    }

    /// Retrieve the canonical path for a struct, enum or function given its **local** id.
    pub fn get_canonical_path_by_local_type_id(
        &mut self,
        used_by_package_id: &PackageId,
        item_id: &rustdoc_types::Id,
    ) -> Result<&[String], anyhow::Error> {
        let definition_package_id =
            self.get_defining_package_id_by_local_type_id(used_by_package_id, item_id)?;
        let type_id = GlobalTypeId::new(item_id.to_owned(), definition_package_id);
        Ok(self.get_canonical_path_by_global_type_id(&type_id))
    }
}

/// Thin wrapper around [`rustdoc_types::Crate`] to:
/// - bundle a derived index (path <> id);
/// - provide query helpers with good error messages.
///
/// It also records the `PackageId` for the corresponding crate within the dependency tree
/// for the workspace it belongs to.
#[derive(Debug, Clone)]
pub struct Crate {
    core: CrateCore,
    path_index: HashMap<Vec<String>, GlobalTypeId>,
    public_local_path_index: HashMap<GlobalTypeId, BTreeSet<Vec<String>>>,
}

#[derive(Debug, Clone)]
struct CrateCore {
    package_id: PackageId,
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
            PackageId::new(external_crate.name.clone())
        } else {
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
            let mut iterator =
                transitive_dependencies.links(guppy::graph::DependencyDirection::Forward);
            iterator
                .find(|link| {
                    link.to().name() == external_crate.name
                        && external_crate_version
                        .as_ref()
                        .map(|v| link.to().version() == v)
                        .unwrap_or(true)
                })
                .ok_or_else(|| {
                    anyhow!(
                        "I could not find the package id for the crate {} among the dependencies of {}",
                        external_crate.name, self.package_id
                    )
                })
                .unwrap()
                .to()
                .id()
                .to_owned()
        }
    }
}

impl Crate {
    fn new(
        collection: &mut CrateCollection,
        krate: rustdoc_types::Crate,
        package_id: PackageId,
    ) -> Self {
        let crate_core = CrateCore { package_id, krate };
        let mut path_index: HashMap<_, _> = crate_core
            .krate
            .paths
            .iter()
            .map(|(id, summary)| {
                (
                    summary.path.clone(),
                    GlobalTypeId::new(id.to_owned(), crate_core.package_id.clone()),
                )
            })
            .collect();

        let mut public_local_path_index = HashMap::new();
        index_local_items(
            &crate_core,
            collection,
            vec![],
            &mut public_local_path_index,
            &crate_core.krate.root,
        );

        path_index.reserve(public_local_path_index.len());
        for (id, public_paths) in &public_local_path_index {
            for public_path in public_paths {
                if path_index.get(public_path).is_none() {
                    path_index.insert(public_path.to_owned(), id.to_owned());
                }
            }
        }

        Self {
            core: crate_core,
            path_index,
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

    pub fn get_type_id_by_path(&self, path: &[String]) -> Result<&GlobalTypeId, UnknownTypePath> {
        self.path_index.get(path).ok_or_else(|| UnknownTypePath {
            type_path: path.to_owned(),
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
    fn get_canonical_path(&self, type_id: &GlobalTypeId) -> &[String] {
        if let Some(path) = self.public_local_path_index.get(type_id) {
            return path.iter().next().unwrap();
        }

        panic!(
            "Failed to find a publicly importable path for the type id `{:?}`. \
             This is likely to be a bug in our handling of rustdoc's JSON output.",
            type_id
        )
    }
}

fn index_local_items<'a>(
    crate_core: &'a CrateCore,
    collection: &mut CrateCollection,
    mut current_path: Vec<&'a str>,
    path_index: &mut HashMap<GlobalTypeId, BTreeSet<Vec<String>>>,
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
                index_local_items(
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
                                    let foreign_item_id = foreign_item_id.raw_id.clone();
                                    // TODO: super-wasteful
                                    let external_core = external_crate.core.clone();
                                    index_local_items(
                                        &external_core,
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
                        index_local_items(
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
        ItemEnum::Struct(_) => {
            let struct_name = current_item
                .name
                .as_deref()
                .expect("All 'struct' items have a 'name' property");
            current_path.push(struct_name);
            let path = current_path.into_iter().map(|s| s.to_string()).collect();
            path_index
                .entry(GlobalTypeId::new(
                    current_item_id.to_owned(),
                    crate_core.package_id.to_owned(),
                ))
                .or_default()
                .insert(path);
        }
        ItemEnum::Enum(_) => {
            let enum_name = current_item
                .name
                .as_deref()
                .expect("All 'enum' items have a 'name' property");
            current_path.push(enum_name);
            let path = current_path.into_iter().map(|s| s.to_string()).collect();
            path_index
                .entry(GlobalTypeId::new(
                    current_item_id.to_owned(),
                    crate_core.package_id.to_owned(),
                ))
                .or_default()
                .insert(path);
        }
        ItemEnum::Function(_) => {
            let function_name = current_item
                .name
                .as_deref()
                .expect("All 'function' items have a 'name' property");
            current_path.push(function_name);
            let path = current_path.into_iter().map(|s| s.to_string()).collect();
            path_index
                .entry(GlobalTypeId::new(
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
pub struct GlobalTypeId {
    raw_id: rustdoc_types::Id,
    package_id: PackageId,
}

impl GlobalTypeId {
    fn new(raw_id: rustdoc_types::Id, package_id: PackageId) -> Self {
        Self { raw_id, package_id }
    }

    pub fn package_id(&self) -> &PackageId {
        &self.package_id
    }
}

#[derive(thiserror::Error, Debug)]
pub struct UnknownTypePath {
    pub type_path: ImportPath,
}

impl Display for UnknownTypePath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let type_path = self.type_path.join("::").replace(' ', "");
        let krate = self.type_path.first().unwrap();
        write!(
            f,
            "I could not find '{type_path}' in the auto-generated documentation for '{krate}'"
        )
    }
}

trait RustdocCrateExt {
    /// Given a crate id, return the corresponding external crate object.
    /// We also try to return the crate version, if we manage to parse it out of the crate HTML
    /// root URL.
    fn get_external_crate_name(&self, crate_id: u32) -> Option<(&ExternalCrate, Option<Version>)>;
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
