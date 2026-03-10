//! Thin wrapper around `rustdoc_processor` that integrates with pavexc's types.

use guppy::graph::PackageGraph;

use rustdoc_processor::cache::RustdocGlobalFsCache;

/// Construct a [`RustdocGlobalFsCache`] pre-configured with Pavex's cache fingerprint
/// and default cache directory (`~/.pavex/rustdoc/cache`).
pub(crate) fn pavex_rustdoc_cache<A: Default + bincode::Decode<()>>(
    toolchain_name: &str,
    cache_workspace_package_docs: bool,
    package_graph: &PackageGraph,
) -> Result<RustdocGlobalFsCache<A>, anyhow::Error> {
    let fingerprint = format!(
        "{}-{}",
        rustdoc_processor::CRATE_VERSION,
        env!("RUSTDOC_CACHE_SOURCE_HASH")
    );
    let cache_dir = xdg_home::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Failed to get the user's home directory"))?
        .join(".pavex/rustdoc/cache");
    RustdocGlobalFsCache::new(
        &fingerprint,
        toolchain_name,
        cache_workspace_package_docs,
        package_graph,
        &cache_dir,
    )
}
