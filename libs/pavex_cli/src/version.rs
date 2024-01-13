//! Utilities to determine which version of `pavexc` should be used.

use anyhow::anyhow;
use guppy::graph::{PackageGraph, PackageSource};
use guppy::Version;

/// Determine which version of `pavex` (the library) is being used
/// in the current workspace.
///
/// It returns an error if the current workspace doesn't have `pavex`
/// in its dependency tree or if it has more than one version of `pavex`.
pub fn pavex_version(
    package_graph: &PackageGraph,
) -> Result<(&Version, PackageSource), PavexVersionError> {
    let pavex_packages: Vec<_> = package_graph
        .packages()
        .filter(|p| p.name() == "pavex")
        .collect();
    if pavex_packages.is_empty() {
        Err(PavexVersionError::NoPavexInDependencyTree)
    } else if pavex_packages.len() > 1 {
        Err(PavexVersionError::MultiplePavexVersions(
            MultiplePavexVersionsError {
                versions: pavex_packages
                    .iter()
                    .map(|p| (p.version().to_owned(), p.source()))
                    .collect(),
            },
        ))
    } else {
        let pavex_package = pavex_packages.first().unwrap();
        Ok((pavex_package.version(), pavex_package.source()))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PavexVersionError<'a> {
    #[error("`pavex` is not in the dependency tree of the current workspace. Are you sure this is a Pavex project?")]
    NoPavexInDependencyTree,
    #[error(transparent)]
    MultiplePavexVersions(MultiplePavexVersionsError<'a>),
}

#[derive(Debug, thiserror::Error)]
#[error("There are multiple versions of `pavex` in the dependency tree of the current workspace: {versions:?}\nIt has to be a single version.")]
pub struct MultiplePavexVersionsError<'a> {
    versions: Vec<(Version, PackageSource<'a>)>,
}

#[tracing::instrument]
fn compute_package_graph() -> Result<PackageGraph, anyhow::Error> {
    // `cargo metadata` seems to be the only reliable way of retrieving the path to
    // the root manifest of the current workspace for a Rust project.
    guppy::MetadataCommand::new()
        .exec()
        .map_err(|e| anyhow!(e))?
        .build_graph()
        .map_err(|e| anyhow!(e))
}
