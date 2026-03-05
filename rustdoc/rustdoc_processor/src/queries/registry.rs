use std::borrow::Cow;

use guppy::PackageId;
use guppy::graph::PackageGraph;
use rustdoc_types::Item;

use super::Crate;
use crate::CannotGetCrateData;
use crate::global_item_id::GlobalItemId;

/// A trait providing cross-crate access during type resolution.
///
/// This abstraction allows `Crate`'s cross-crate methods to work
/// without depending on a concrete collection type.
pub trait CrateRegistry {
    /// Return the package graph for the current workspace.
    fn package_graph(&self) -> &PackageGraph;
    /// Get or compute the `Crate` for the given `PackageId`.
    fn get_or_compute_crate(&self, package_id: &PackageId) -> Result<&Crate, CannotGetCrateData>;

    /// Retrieve type information given its [`GlobalItemId`].
    ///
    /// It panics if no item is found for the specified [`GlobalItemId`].
    fn get_item_by_global_type_id(&self, type_id: &GlobalItemId) -> Cow<'_, Item> {
        let krate = self.get_or_compute_crate(&type_id.package_id).unwrap();
        krate.get_item_by_local_type_id(&type_id.rustdoc_item_id)
    }

    /// Retrieve the canonical path for a struct, enum or function given its [`GlobalItemId`].
    ///
    /// It panics if no item is found for the specified [`GlobalItemId`].
    fn get_canonical_path_by_global_type_id(
        &self,
        type_id: &GlobalItemId,
    ) -> Result<&[String], anyhow::Error> {
        let krate = self.get_or_compute_crate(&type_id.package_id).unwrap();
        krate.get_canonical_path(type_id)
    }

    /// Retrieve the canonical path and the [`GlobalItemId`] for a struct, enum or function given
    /// its **local** id.
    fn get_canonical_path_by_local_type_id(
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
            let used_by_krate = self.get_or_compute_crate(used_by_package_id)?;
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
        let definition_krate = self.get_or_compute_crate(&definition_package_id)?;
        let type_id = definition_krate.get_item_id_by_path(&path, self)??;
        let canonical_path = self.get_canonical_path_by_global_type_id(&type_id)?;
        Ok((type_id.clone(), canonical_path))
    }
}
