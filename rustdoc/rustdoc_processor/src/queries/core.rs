use guppy::PackageId;
use guppy::graph::PackageGraph;

use super::resolution::compute_package_id_for_crate_id;
use crate::crate_data::CrateData;

#[derive(Debug, Clone)]
pub struct CrateCore {
    /// The `PackageId` for the corresponding crate within the dependency tree
    /// for the workspace it belongs to.
    pub package_id: PackageId,
    /// The JSON documentation for the crate.
    pub krate: CrateData,
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
        package_graph: &PackageGraph,
        maybe_dependent_crate_name: Option<&str>,
    ) -> Result<PackageId, anyhow::Error> {
        compute_package_id_for_crate_id(
            &self.package_id,
            &self.krate.external_crates,
            crate_id,
            maybe_dependent_crate_name,
            package_graph,
        )
    }
}
