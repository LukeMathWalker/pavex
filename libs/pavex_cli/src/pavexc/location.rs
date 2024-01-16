use crate::locator::ToolchainsLocator;
use crate::pavexc::install::UnsupportedSourceError;
use guppy::graph::{ExternalSource, PackageGraph, PackageSource};
use guppy::Version;
use std::path::PathBuf;

/// Given the version and source for the `pavex` library crate, determine the path to the
/// `pavexc` binary that should be used.
pub(super) fn path_from_graph(
    toolchains_locator: &ToolchainsLocator,
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
                    Ok(Ok(toolchains_locator
                        .registry()
                        .toolchain_dir(c, version)
                        .pavexc()))
                }
                ExternalSource::Git {
                    repository,
                    resolved,
                    ..
                } => Ok(Ok(toolchains_locator
                    .git()
                    .toolchain_dir(repository, resolved)
                    .pavexc())),
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
