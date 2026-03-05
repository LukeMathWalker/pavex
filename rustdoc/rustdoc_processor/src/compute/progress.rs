use guppy::PackageId;
use guppy::graph::PackageGraph;

/// Callback for reporting progress during `cargo rustdoc` invocations.
pub trait ComputeProgress {
    fn before_computing(&self, package_graph: &PackageGraph, package_ids: &[PackageId]);
    fn after_computed(
        &self,
        package_graph: &PackageGraph,
        package_ids: &[PackageId],
        duration: std::time::Duration,
    );
}

/// No-op implementation.
pub struct NoProgress;

impl ComputeProgress for NoProgress {
    fn before_computing(&self, _: &PackageGraph, _: &[PackageId]) {}
    fn after_computed(&self, _: &PackageGraph, _: &[PackageId], _: std::time::Duration) {}
}
