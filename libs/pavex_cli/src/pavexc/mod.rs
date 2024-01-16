use crate::locator::PavexLocator;
use anyhow::Context;
use cargo_like_utils::shell::Shell;
use fs_err::PathExt;
use guppy::graph::{PackageGraph, PackageSource};
use semver::Version;
use std::path::{Path, PathBuf};

mod install;
mod location;
mod prebuilt;
mod version;

static PAVEX_GITHUB_URL: &str = "https://github.com/LukeMathWalker/pavex";

/// Get the path to the `pavexc` binary that matches
/// the version of the `pavex` library crate used in the current workspace.
///
/// If necessary, it'll install the binary first, either by downloading
/// a pre-built binary or by building it from source.
pub fn get_or_install_from_graph(
    shell: &mut Shell,
    locator: &PavexLocator,
    package_graph: &PackageGraph,
) -> Result<PathBuf, anyhow::Error> {
    let (version, package_source) = version::pavex_lib_version(package_graph)?;
    let pavexc_cli_path = location::path_from_graph(
        &locator.toolchains(),
        package_graph,
        version,
        &package_source,
    )??;
    _install(shell, &pavexc_cli_path, version, &package_source)?;
    Ok(pavexc_cli_path)
}

/// Install a given version of the Pavex CLI from GitHub.
pub fn get_or_install_from_version(
    shell: &mut Shell,
    locator: &PavexLocator,
    version: &Version,
) -> Result<PathBuf, anyhow::Error> {
    let revision_sha = crate::env::commit_sha();
    let pavexc_path = locator
        .toolchains()
        .git()
        .toolchain_dir(PAVEX_GITHUB_URL, revision_sha)
        .pavexc();
    _install(
        shell,
        &pavexc_path,
        version,
        &PackageSource::External(&format!("git+{PAVEX_GITHUB_URL}#{revision_sha}",)),
    )?;
    Ok(pavexc_path)
}

fn _install(
    shell: &mut Shell,
    pavexc_cli_path: &Path,
    version: &Version,
    package_source: &PackageSource,
) -> Result<(), anyhow::Error> {
    if let Ok(true) = pavexc_cli_path.try_exists() {
        let metadata = pavexc_cli_path
            .fs_err_metadata()
            .context("Failed to get file metadata for the `pavexc` binary")?;
        if metadata.is_file() {
            return Ok(());
        }
    }

    if let Some(parent_dir) = pavexc_cli_path.parent() {
        fs_err::create_dir_all(parent_dir).context("Failed to create binary cache directory")?;
    }

    install::install(shell, &pavexc_cli_path, version, &package_source)?;
    #[cfg(unix)]
    executable::make_executable(&pavexc_cli_path)?;
    Ok(())
}

#[cfg(unix)]
mod executable {
    use anyhow::Context;
    use std::os::unix::fs::PermissionsExt;

    pub(super) fn make_executable(pavexc_cli_path: &std::path::Path) -> Result<(), anyhow::Error> {
        let mut perms = pavexc_cli_path
            .metadata()
            .context("Failed to get file metadata for the `pavexc` binary")?
            .permissions();
        // Add the executable permission to the owner of the file
        perms.set_mode(perms.mode() | 0o100);
        std::fs::set_permissions(&pavexc_cli_path, perms)
            .context("Failed to set permissions for the `pavexc` binary")?;
        Ok(())
    }
}
