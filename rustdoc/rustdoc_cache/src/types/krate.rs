use std::borrow::Cow;
use std::sync::{Arc, RwLock};

use ahash::HashMap;
use anyhow::anyhow;
use guppy::PackageId;
use guppy::graph::PackageGraph;
use rustdoc_types::Item;

use crate::CannotGetCrateData;
use super::{
    CrateCore, CrateIdNeedle, ExternalReExport, ExternalReExports, GlobalItemId, ImportIndex,
    ImportPath2Id, UnknownItemPath,
};
use super::registry::CrateRegistry;

/// Thin wrapper around [`rustdoc_types::Crate`] to:
/// - bundle derived indexes;
/// - provide query helpers with good error messages.
///
/// It also records the `PackageId` for the corresponding crate within the dependency tree
/// for the workspace it belongs to.
#[derive(Debug, Clone)]
pub struct Crate {
    pub core: CrateCore,
    /// An index to lookup the id of a type given one of its import paths, either
    /// public or private.
    ///
    /// The index does NOT contain macros, since macros and types live in two
    /// different namespaces and can contain items with the same name.
    /// E.g. `core::clone::Clone` is both a trait and a derive macro.
    pub import_path2id: ImportPath2Id,
    /// Types (or modules!) re-exported from other crates.
    pub external_re_exports: ExternalReExports,
    /// An in-memory index of all modules, traits, structs, enums, and functions that were defined in the current crate.
    ///
    /// It can be used to retrieve all publicly visible items as well as computing a "canonical path"
    /// for each of them.
    pub import_index: ImportIndex,
    /// An internal cache to avoid traversing the package graph every time we need to
    /// translate a crate id into a package id via [`Self::compute_package_id_for_crate_id`]
    /// or [`Self::compute_package_id_for_crate_id_with_hint`].
    pub crate_id2package_id: Arc<RwLock<HashMap<CrateIdNeedle, PackageId>>>,
}

impl Crate {
    /// Create a new `Crate` from its constituent parts.
    pub fn new(
        core: CrateCore,
        import_path2id: ImportPath2Id,
        external_re_exports: ExternalReExports,
        import_index: ImportIndex,
    ) -> Self {
        Self {
            core,
            import_path2id,
            external_re_exports,
            import_index,
            crate_id2package_id: Default::default(),
        }
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
        registry: &impl CrateRegistry,
    ) -> Result<PackageId, anyhow::Error> {
        self.compute_package_id_for_crate_id_with_hint(crate_id, registry, None)
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
        registry: &impl CrateRegistry,
        maybe_dependent_crate_name: Option<&str>,
    ) -> Result<PackageId, anyhow::Error> {
        let needle = CrateIdNeedle {
            crate_id,
            maybe_dependent_crate_name: maybe_dependent_crate_name.map(|s| s.to_owned()),
        };
        // Check the cache first.
        if let Some(package_id) = self.crate_id2package_id.read().unwrap().get(&needle) {
            return Ok(package_id.to_owned());
        }

        // If we don't have a cached entry, perform the graph traversal.
        let outcome = self.core.compute_package_id_for_crate_id(
            crate_id,
            registry.package_graph(),
            maybe_dependent_crate_name,
        );

        // If successful, cache the outcome.
        if let Ok(outcome) = &outcome {
            self.crate_id2package_id
                .write()
                .unwrap()
                .insert(needle, outcome.to_owned());
        }
        outcome
    }

    pub fn get_item_id_by_path(
        &self,
        path: &[String],
        registry: &impl CrateRegistry,
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
                    .compute_package_id_for_crate_id(
                        *external_crate_id,
                        registry.package_graph(),
                        None,
                    )
                    .unwrap();
                let source_krate = registry
                    .get_or_compute_crate(&source_package_id)
                    .unwrap();
                if let Ok(source_id) =
                    source_krate.get_item_id_by_path(&original_source_path, registry)
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
    pub fn get_summary_by_local_type_id(
        &self,
        id: &rustdoc_types::Id,
    ) -> Result<Cow<'_, rustdoc_types::ItemSummary>, anyhow::Error> {
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
    pub fn get_canonical_path(
        &self,
        type_id: &GlobalItemId,
    ) -> Result<&[String], anyhow::Error> {
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
