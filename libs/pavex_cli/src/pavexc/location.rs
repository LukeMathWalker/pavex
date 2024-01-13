use crate::pavexc::install::UnsupportedSourceError;
use guppy::graph::{ExternalSource, PackageGraph, PackageSource};
use guppy::Version;
use sha2::Digest;
use std::path::PathBuf;

/// Given the version and source for the `pavex` library crate, determine the path to the
/// `pavexc` binary that should be used.
pub(super) fn pavexc_cli_path(
    package_graph: &PackageGraph,
    version: &Version,
    package_source: &PackageSource,
) -> Result<Result<PathBuf, UnsupportedSourceError>, anyhow::Error> {
    match package_source {
        PackageSource::Workspace(_) => {
            let workspace_root = package_graph.workspace().root();
            let pavexc_cli_path = workspace_root.join("target").join("release").join("pavexc");
            Ok(Ok(pavexc_cli_path.into_std_path_buf()))
        }
        PackageSource::Path(p) => {
            let workspace_root = p
                .parent()
                .expect("pavex's source path has to have a parent");
            let pavexc_cli_path = workspace_root.join("target").join("release").join("pavexc");
            Ok(Ok(pavexc_cli_path.into_std_path_buf()))
        }
        PackageSource::External(_) => {
            let parsed = package_source.parse_external();
            let Some(parsed) = parsed else {
                return Ok(Err(UnsupportedSourceError {
                    package_source: package_source.to_string(),
                    version: version.to_owned(),
                }));
            };
            let bin_cache_dir = xdg_home::home_dir()
                .ok_or_else(|| anyhow::anyhow!("Failed to get the path to your home directory"))?
                .join(".pavex")
                .join("pavexc");
            match parsed {
                ExternalSource::Registry(c) => {
                    if !package_source.is_crates_io() {
                        return Ok(Err(UnsupportedSourceError {
                            package_source: format!(
                                "a private registry ({})",
                                c.strip_prefix("registry+").unwrap_or(c)
                            ),
                            version: version.to_owned(),
                        }));
                    }
                    Ok(Ok(bin_cache_dir.join(format!("{version}")).join("pavexc")))
                }
                ExternalSource::Git {
                    repository,
                    resolved,
                    ..
                } => {
                    let repository_hash = sha2::Sha256::digest(repository.as_bytes());
                    // Take the first 7 hex digits of the hash
                    let repository_hash = format!("{:x}", repository_hash)
                        .chars()
                        .take(7)
                        .collect::<String>();
                    // Take the first 7 hex digits of the hash, i.e. git's short commit SHA
                    let resolved = resolved.chars().take(7).collect::<String>();
                    Ok(Ok(bin_cache_dir.join(format!(
                        "git/{repository_hash}/{version}/{resolved}/pavexc"
                    ))))
                }
                _ => {
                    return Ok(Err(UnsupportedSourceError {
                        package_source: package_source.to_string(),
                        version: version.to_owned(),
                    }));
                }
            }
        }
    }
}
