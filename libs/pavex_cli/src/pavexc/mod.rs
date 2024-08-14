use crate::locator::PavexLocator;
use crate::pavexc::install::{GitReq, InstallSource};
use crate::version::latest_released_version;
use anyhow::Context;
use cargo_like_utils::shell::Shell;
use fs_err::PathExt;
use guppy::graph::PackageGraph;
use semver::Version;
use std::path::{Path, PathBuf};

mod install;
mod location;
mod setup;
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
    let pavex_cli_version = Version::parse(env!("CARGO_PKG_VERSION")).context(
        "Failed to parse the version of `pavex` CLI. Are you using a fork of `pavex_cli`?",
    )?;
    let (pavex_lib_version, package_source) = version::pavex_lib_version(package_graph).context(
        "Failed to determine the version of the `pavex` library crate in this workspace.",
    )?;
    if pavex_lib_version > &pavex_cli_version {
        let latest_pavex_cli_version = latest_released_version()?;
        // There is a delay between the release of the `pavex` library
        // and the release of pre-built `pavex` CLI binaries for the same version.
        //
        // We don't want to block users from building their workspace just because
        // the pre-built binary for the new version of `pavex` CLI is not available yet.
        if &latest_pavex_cli_version >= pavex_lib_version {
            return Err(anyhow::anyhow!(
            "Your `pavex` CLI is too old: \
            the current workspace uses version `{}` of the `pavex` library, but you're using version `{}` of the `pavex` CLI.\n\
            You must update your `pavex` CLI to a version greater or equal than `{}` to build the current workspace. \n\
            To fix the issue, run:\n\n    pavex self update\n\n\
            It'll update your `pavex` CLI to the latest released version.",
            pavex_lib_version,
            pavex_cli_version,
            pavex_lib_version,
        ));
        }
    }
    let pavexc_cli_path = location::path_from_graph(
        &locator.toolchains(),
        package_graph,
        pavex_lib_version,
        &package_source,
    )
    .context("Failed to determine where the `pavexc` binary should be located")??;
    _install(
        shell,
        &pavexc_cli_path,
        pavex_lib_version,
        &package_source.try_into()?,
    )
    .context("Failed to get or install the `pavexc` binary")?;
    Ok(pavexc_cli_path)
}

/// Install a given version of the Pavex CLI from GitHub.
pub fn get_or_install_from_version(
    shell: &mut Shell,
    locator: &PavexLocator,
    version: &Version,
) -> Result<PathBuf, anyhow::Error> {
    let pavexc_path = locator
        .toolchains()
        .registry()
        // This is not quite true since we're installing from our GitHub repository
        // and not from crates.io, but it's going to be good enough until we publish
        // the Pavex CLI on crates.io.
        .toolchain_dir(guppy::graph::ExternalSource::CRATES_IO_URL, version)
        .pavexc();
    _install(
        shell,
        &pavexc_path,
        version,
        &InstallSource::External(install::ExternalSource::Git {
            repository: PAVEX_GITHUB_URL.to_owned(),
            req: GitReq::Tag(version.to_string()),
            resolved: None,
        }),
    )?;
    Ok(pavexc_path)
}

fn _install(
    shell: &mut Shell,
    pavexc_cli_path: &Path,
    version: &Version,
    install_source: &InstallSource,
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

    install::install(shell, pavexc_cli_path, version, install_source)?;
    #[cfg(unix)]
    executable::make_executable(pavexc_cli_path)?;
    setup::pavexc_setup(pavexc_cli_path)
        .context("Failed to install the nightly Rust toolchain required by `pavexc`")?;
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
        std::fs::set_permissions(pavexc_cli_path, perms)
            .context("Failed to set permissions for the `pavexc` binary")?;
        Ok(())
    }
}
