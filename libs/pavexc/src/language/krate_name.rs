use guppy::{PackageId, graph::PackageGraph};

use crate::rustdoc::TOOLCHAIN_CRATES;

// Keeping track of what returns a normalized crate name vs a "raw" crate name is a mess,
// therefore we normalize everything as a sanity measure.
fn normalize(crate_name: &str) -> String {
    crate_name.replace('-', "_")
}

/// Find the package ID for a given crate name within the package graph.
pub fn krate_name2package_id(
    name: &str,
    graph: &PackageGraph,
) -> Result<PackageId, UnknownCrateName> {
    let name = normalize(&name);
    if TOOLCHAIN_CRATES.contains(&name.as_str()) {
        return Ok(PackageId::new(name));
    }
    // TODO: What if we have more than a single package with the same name?
    let Some(package) = graph.packages().find(|p| normalize(p.name()) == name) else {
        return Err(UnknownCrateName(name.into()).into());
    };
    Ok(package.id().to_owned())
}

/// Find the package ID for a given dependency name.
///
/// The search is anchored by `used_in`, the name of the crate where that name was used.
pub fn dependency_name2package_id(
    name: &str,
    used_in: &str,
    graph: &PackageGraph,
) -> Result<PackageId, CrateNameResolutionError> {
    let used_in = normalize(used_in);
    let name = normalize(name);

    let used_in_id = krate_name2package_id(&used_in, graph)?;
    if used_in == name {
        return Ok(used_in_id);
    }
    let used_in_package = graph
        .metadata(&used_in_id)
        // This could happen if `used_in_id` was a toolchain crate id,
        // but I doubt `std` will ever register Pavex components directly,
        // so we can safely assume that the package metadata exists.
        .expect("Failed to resolve package id to package metadata");

    match used_in_package
        .direct_links()
        .find(|d| normalize(d.resolved_name()) == name)
    {
        Some(dependency) => Ok(dependency.to().id().to_owned()),
        _ => {
            if TOOLCHAIN_CRATES.contains(&name.as_str()) {
                Ok(PackageId::new(name))
            } else {
                Err(UnknownDependency {
                    dependent_name: used_in.to_string(),
                    dependency_name: name.to_string(),
                }
                .into())
            }
        }
    }
}

/// The various ways in which a crate name can fail to be resolved.
#[derive(Debug, thiserror::Error, Clone)]
pub enum CrateNameResolutionError {
    #[error(transparent)]
    UnknownCrateName(#[from] UnknownCrateName),
    #[error(transparent)]
    UnknownDependency(#[from] UnknownDependency),
}

#[derive(Debug, thiserror::Error, Clone)]
#[error("There is no package in the dependency tree named `{0}`")]
pub struct UnknownCrateName(pub String);

#[derive(Debug, thiserror::Error, Clone)]
#[error(
    "There is no package named `{dependency_name}` among the dependencies of `{dependent_name}`."
)]
pub struct UnknownDependency {
    pub dependent_name: String,
    pub dependency_name: String,
}
