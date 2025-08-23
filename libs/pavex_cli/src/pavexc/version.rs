//! Utilities to determine which version of `pavexc` should be used.
use guppy::Version;
use guppy::graph::{PackageGraph, PackageSource};

/// Determine which version of `pavex` (the library) is being used
/// in the current workspace.
///
/// It returns an error if the current workspace doesn't have `pavex`
/// in its dependency tree or if it has more than one version of `pavex`.
pub(super) fn pavex_lib_version(
    package_graph: &PackageGraph,
) -> Result<(&Version, PackageSource<'_>), PavexVersionError> {
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
                    .map(|p| (p.version().to_owned(), p.source().to_string()))
                    .collect(),
            },
        ))
    } else {
        let pavex_package = pavex_packages.first().unwrap();
        Ok((pavex_package.version(), pavex_package.source()))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PavexVersionError {
    #[error(
        "`pavex` is not in the dependency tree of the current workspace. Are you sure this is a Pavex project?"
    )]
    NoPavexInDependencyTree,
    #[error(transparent)]
    MultiplePavexVersions(MultiplePavexVersionsError),
}

#[derive(Debug, thiserror::Error)]
#[error(
    "There are multiple versions of `pavex` in the dependency tree of the current workspace: {versions:?}\nIt has to be a single version."
)]
pub struct MultiplePavexVersionsError {
    versions: Vec<(Version, String)>,
}
