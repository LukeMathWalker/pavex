use guppy::PackageId;
use guppy::graph::PackageGraph;

use crate::CannotGetCrateData;
use super::krate::Crate;

/// A trait providing cross-crate access during type resolution.
///
/// This abstraction allows `Crate`'s cross-crate methods to work
/// without depending on a concrete collection type.
pub trait CrateRegistry {
    /// Return the package graph for the current workspace.
    fn package_graph(&self) -> &PackageGraph;
    /// Get or compute the `Crate` for the given `PackageId`.
    fn get_or_compute_crate(&self, package_id: &PackageId) -> Result<&Crate, CannotGetCrateData>;
}
