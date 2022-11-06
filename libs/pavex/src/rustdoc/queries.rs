use std::collections::{BTreeSet, HashMap};
use std::fmt::{Display, Formatter};

use anyhow::anyhow;
use guppy::graph::PackageGraph;
use guppy::{PackageId, Version};
use rustdoc_types::{ItemEnum, Visibility};

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
pub struct CrateCollection(HashMap<String, Crate>, PackageGraph);

impl CrateCollection {
    /// Initialise the collection for a `PackageGraph`.
    pub fn new(package_graph: PackageGraph) -> Self {
        Self(Default::default(), package_graph)
    }

    /// Compute the documentation for the crate associated with a specific `PackageId`.
    ///
    /// It will be retrieved from [`CrateCollection`]'s internal cache if it was computed before.
    pub fn get_or_compute_by_id(
        &mut self,
        package_id: &PackageId,
    ) -> Result<&Crate, CannotGetCrateData> {
        let package_spec = if TOOLCHAIN_CRATES.contains(&package_id.repr()) {
            PackageIdSpecification {
                source: None,
                name: package_id.repr().to_string(),
                version: None,
            }
        } else {
            let package_metadata = self.1.metadata(package_id).expect("Unknown package ID");
            PackageIdSpecification::new(&package_metadata)
        };
        if self.0.get(&package_spec.to_string()).is_none() {
            let krate = get_crate_data(
                self.1.workspace().target_directory().as_std_path(),
                &package_spec,
            )?;
            let krate = Crate::new(krate, package_id.to_owned());
            self.0.insert(package_spec.to_string(), krate);
        }
        Ok(&self.0[&package_spec.to_string()])
    }

    /// Retrieve the package id where a certain item was originally defined.
    pub fn get_defining_package_id_for_item(
        &mut self,
        used_by_package_id: &PackageId,
        item_id: &rustdoc_types::Id,
    ) -> Result<PackageId, anyhow::Error> {
        let package_graph = self.1.clone();
        let used_by_krate = self.get_or_compute_by_id(used_by_package_id)?;
        let type_summary = used_by_krate.get_summary_by_id(item_id)?;

        let type_package_id = if type_summary.crate_id == 0 {
            used_by_krate.package_id().to_owned()
        } else {
            let (owning_crate, owning_crate_version) = used_by_krate
                .get_external_crate_name(type_summary.crate_id)
                .unwrap();
            if TOOLCHAIN_CRATES.contains(&owning_crate.name.as_str()) {
                PackageId::new(owning_crate.name.clone())
            } else {
                let transitive_dependencies = package_graph
                    .query_forward([used_by_package_id])
                    .unwrap()
                    .resolve();
                let mut iterator =
                    transitive_dependencies.links(guppy::graph::DependencyDirection::Forward);
                iterator
                    .find(|link| {
                        link.to().name() == owning_crate.name
                            && owning_crate_version
                                .as_ref()
                                .map(|v| link.to().version() == v)
                                .unwrap_or(true)
                    })
                    .ok_or_else(|| {
                        anyhow!(
                            "I could not find the package id for the crate where `{}` is defined",
                            type_summary.path.join("::")
                        )
                    })
                    .unwrap()
                    .to()
                    .id()
                    .to_owned()
            }
        };
        Ok(type_package_id)
    }

    pub fn get_canonical_import_path(
        &mut self,
        used_by_package_id: &PackageId,
        item_id: &rustdoc_types::Id,
    ) -> Result<Vec<String>, anyhow::Error> {
        let definition_package_id =
            self.get_defining_package_id_for_item(used_by_package_id, item_id)?;

        let used_by_krate = self.get_or_compute_by_id(used_by_package_id)?;
        let type_summary = used_by_krate.get_summary_by_id(item_id)?;
        let referenced_base_type_path = type_summary.path.clone();
        let base_type = if type_summary.crate_id == 0 {
            self.get_or_compute_by_id(used_by_package_id)?
                .get_importable_path(item_id)
        } else {
            // The crate where the type is actually defined.
            let source_crate = self.get_or_compute_by_id(&definition_package_id)?;
            let type_definition_id = source_crate.get_id_by_path(&referenced_base_type_path)?;
            source_crate.get_importable_path(type_definition_id)
        }
        .to_owned();
        Ok(base_type)
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
    package_id: PackageId,
    krate: rustdoc_types::Crate,
    path_index: HashMap<Vec<String>, rustdoc_types::Id>,
    public_local_path_index: HashMap<rustdoc_types::Id, BTreeSet<Vec<String>>>,
}

impl Crate {
    fn new(krate: rustdoc_types::Crate, package_id: PackageId) -> Self {
        let mut path_index: HashMap<_, _> = krate
            .paths
            .iter()
            .map(|(id, summary)| (summary.path.clone(), id.to_owned()))
            .collect();

        let mut public_local_path_index = HashMap::new();
        index_local_items(&krate, vec![], &mut public_local_path_index, &krate.root);

        path_index.reserve(public_local_path_index.len());
        for (id, public_paths) in &public_local_path_index {
            for public_path in public_paths {
                if path_index.get(public_path).is_none() {
                    path_index.insert(public_path.to_owned(), id.to_owned());
                }
            }
        }

        Self {
            package_id,
            krate,
            path_index,
            public_local_path_index,
        }
    }

    /// Given a crate id, return the corresponding external crate object.
    /// We also try to return the crate version, if we manage to parse it out of the crate HTML
    /// root URL.
    pub fn get_external_crate_name(
        &self,
        crate_id: u32,
    ) -> Option<(&rustdoc_types::ExternalCrate, Option<Version>)> {
        let external_crate = self.krate.external_crates.get(&crate_id);
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

    pub fn package_id(&self) -> &PackageId {
        &self.package_id
    }

    pub fn get_id_by_path(&self, path: &[String]) -> Result<&rustdoc_types::Id, UnknownTypePath> {
        self.path_index.get(path).ok_or_else(|| UnknownTypePath {
            type_path: path.to_owned(),
        })
    }

    pub fn get_summary_by_id(
        &self,
        id: &rustdoc_types::Id,
    ) -> Result<&rustdoc_types::ItemSummary, anyhow::Error> {
        self.krate.paths.get(id).ok_or_else(|| {
            anyhow!(
                "Failed to look up the type id `{}` in the rustdoc's path index. \
                This is likely to be a bug in rustdoc's JSON output.",
                id.0
            )
        })
    }

    pub fn get_type_by_id(&self, id: &rustdoc_types::Id) -> &rustdoc_types::Item {
        let type_ = self.krate.index.get(id);
        if type_.is_none() {
            panic!(
                "Failed to look up the type id `{}` in the rustdoc's index. \
                This is likely to be a bug in rustdoc's JSON output.",
                id.0
            )
        }
        type_.unwrap()
    }

    pub fn get_type_by_path(
        &self,
        path: &[String],
    ) -> Result<&rustdoc_types::Item, UnknownTypePath> {
        let id = self.get_id_by_path(path)?;
        Ok(self.get_type_by_id(id))
    }

    pub fn get_importable_path(&self, id: &rustdoc_types::Id) -> &[String] {
        if let Some(path) = self.public_local_path_index.get(id) {
            return path.iter().next().unwrap();
        }

        let item = self.get_type_by_id(id);
        if item.crate_id != 0 {
            let external_crate = &self.krate.external_crates[&item.crate_id];
            panic!(
                "You can only retrieve a path that is guaranteed to be public for local types. \
                `{}` is not local. That id belongs to {} (crate_id={}).",
                &item.id.0, &external_crate.name, item.crate_id
            )
        }

        panic!(
            "Failed to find a publicly importable path for the type id `{}`. \
             This is likely to be a bug in our handling of rustdoc's JSON output.",
            id.0
        )
    }
}

fn index_local_items<'a>(
    krate: &'a rustdoc_types::Crate,
    mut current_path: Vec<&'a str>,
    path_index: &mut HashMap<rustdoc_types::Id, BTreeSet<Vec<String>>>,
    current_item_id: &rustdoc_types::Id,
) {
    // TODO: the way we handle `current_path` is extremely wasteful,
    // we can likely reuse the same buffer throughout.
    let current_item = &krate.index[current_item_id];

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
                index_local_items(krate, current_path.clone(), path_index, item_id);
            }
        }
        ItemEnum::Import(i) => {
            if let Some(imported_id) = &i.id {
                match krate.index.get(imported_id) {
                    None => {
                        if let Some(imported_summary) = krate.paths.get(imported_id) {
                            debug_assert!(imported_summary.crate_id != 0);
                        } else {
                            // TODO: this is firing for std's JSON docs. File a bug report.
                            // panic!("The imported id ({}) is not listed in the index nor in the path section of rustdoc's JSON output", imported_id.0)
                        }
                    }
                    Some(imported_item) => {
                        if let ItemEnum::Module(_) = imported_item.inner {
                            current_path.push(&i.name);
                        }
                        index_local_items(krate, current_path.clone(), path_index, imported_id);
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
                .entry(current_item_id.to_owned())
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
                .entry(current_item_id.to_owned())
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
                .entry(current_item_id.to_owned())
                .or_default()
                .insert(path);
        }
        _ => {}
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
