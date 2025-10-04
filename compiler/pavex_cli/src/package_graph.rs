use anyhow::Context;
use guppy::graph::PackageGraph;

pub fn compute_package_graph() -> Result<PackageGraph, anyhow::Error> {
    let metadata = tracing::info_span!("Invoke 'cargo metadata'")
        .in_scope(|| guppy::MetadataCommand::new().exec())
        .context("Failed to invoke `cargo metadata`")?;
    let graph = tracing::info_span!("Build package graph")
        .in_scope(|| metadata.build_graph())
        .context("Failed to build package graph")?;
    Ok(graph)
}
