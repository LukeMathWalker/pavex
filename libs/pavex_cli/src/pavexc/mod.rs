use anyhow::Context;
use guppy::graph::PackageGraph;
use std::path::PathBuf;

mod install;
mod location;
mod prebuilt;
mod version;

/// Get the path to the `pavexc` binary that matches
/// the version of the `pavex` library crate used in the current workspace.
///
/// If necessary, it'll install the binary first, either by downloading
/// a pre-built binary or by building it from source.
pub fn get_or_install_pavexc_cli(package_graph: &PackageGraph) -> Result<PathBuf, anyhow::Error> {
    let (version, package_source) = version::pavex_version(package_graph)?;
    let pavexc_cli_path = location::pavexc_cli_path(package_graph, version, &package_source)??;
    if pavexc_cli_path.exists() {
        Ok(pavexc_cli_path)
    } else {
        if let Some(parent_dir) = pavexc_cli_path.parent() {
            fs_err::create_dir_all(parent_dir)
                .context("Failed to create binary cache directory")?;
        }

        install::install_pavexc_cli(&pavexc_cli_path, version, &package_source)?;
        #[cfg(unix)]
        executable::make_executable(&pavexc_cli_path)?;
        Ok(pavexc_cli_path)
    }
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
