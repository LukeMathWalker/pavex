use anyhow::anyhow;
use guppy::graph::PackageGraph;

#[tracing::instrument]
pub fn compute_package_graph() -> Result<PackageGraph, anyhow::Error> {
    // `cargo metadata` seems to be the only reliable way of retrieving the path to
    // the root manifest of the current workspace for a Rust project.
    guppy::MetadataCommand::new()
        .exec()
        .map_err(|e| anyhow!(e))?
        .build_graph()
        .map_err(|e| anyhow!(e))
}
