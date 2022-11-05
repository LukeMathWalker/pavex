use std::path::{Path, PathBuf};

use anyhow::Context;

use crate::rustdoc::package_id_spec::PackageIdSpecification;
use crate::rustdoc::TOOLCHAIN_CRATES;

#[derive(Debug, thiserror::Error)]
#[error("I failed to retrieve information about the public types of a package in your workspace ('{package_spec}').")]
pub struct CannotGetCrateData {
    pub package_spec: String,
    #[source]
    pub source: anyhow::Error,
}

/// Return the JSON documentation for a crate.
///
/// It is computed on the fly for crates that are local to the current workspace.
/// It is retrieved via `rustup` for toolchain crates (e.g. `std`).
pub fn get_crate_data(
    root_folder: &Path,
    package_id_spec: &PackageIdSpecification,
) -> Result<rustdoc_types::Crate, CannotGetCrateData> {
    // Some crates are not compiled as part of the dependency tree of the current workspace.
    // They are instead bundled as part of Rust's toolchain and automatically available for import
    // and usage in your crate: the standard library (`std`), `core` (a smaller subset of `std`
    // that does not require an allocator), `alloc` (a smaller subset of `std` that assumes you
    // can allocate).
    // Since those crates are pre-compiled (and somewhat special), we can't generate their
    // documentation on the fly. We assume that their JSON docs have been pre-computed and are
    // available for us to look at.
    if TOOLCHAIN_CRATES.contains(&package_id_spec.name.as_str()) {
        get_toolchain_crate_data(package_id_spec)
    } else {
        _get_crate_data(root_folder, package_id_spec)
    }
    .map_err(|e| CannotGetCrateData {
        package_spec: package_id_spec.to_string(),
        source: e,
    })
}

fn get_toolchain_crate_data(
    package_id_spec: &PackageIdSpecification,
) -> Result<rustdoc_types::Crate, anyhow::Error> {
    let root_folder = get_json_docs_root_folder_via_rustup()?;
    let json_path = root_folder.join(format!("{}.json", package_id_spec.name));
    let json = fs_err::read_to_string(json_path).with_context(|| {
        format!(
            "Failed to retrieve the JSON docs for {}",
            package_id_spec.name
        )
    })?;
    serde_json::from_str::<rustdoc_types::Crate>(&json)
        .with_context(|| {
            format!(
                "Failed to deserialize the JSON docs for {}",
                package_id_spec.name
            )
        })
        .map_err(Into::into)
}

fn get_json_docs_root_folder_via_rustup() -> Result<PathBuf, anyhow::Error> {
    let nightly_toolchain = get_nightly_toolchain_root_folder_via_rustup()?;
    Ok(nightly_toolchain.join("share/doc/rust/json"))
}

/// In order to determine where all components attached to the nightly toolchain are stored,
/// we ask `rustup` to tell us where the `cargo` binary its location.
/// It looks like its location is always going to be `<toolchain root folder>/bin/cargo`, so
/// we compute `<toolchain root folder>` by chopping off the final two components of the path
/// returned by `rustup`.
fn get_nightly_toolchain_root_folder_via_rustup() -> Result<PathBuf, anyhow::Error> {
    let mut cmd = std::process::Command::new("rustup");
    cmd.arg("which")
        .arg("--toolchain")
        .arg("nightly")
        .arg("cargo");

    let output = cmd.output().with_context(|| {
        format!(
            "Failed to run a `rustup` command. Is `rustup` installed?\n{:?}",
            cmd
        )
    })?;

    if !output.status.success() {
        anyhow::bail!(
            "An invocation of `rustup` exited with non-zero status code.\n{:?}",
            cmd
        );
    }
    let path = std::str::from_utf8(&output.stdout)
        .with_context(|| {
            format!(
                "An invocation of `rustup` returned non-UTF8 data as output.\n{:?}",
                cmd
            )
        })?
        .trim();
    let path = Path::new(path);
    debug_assert!(
        path.ends_with("bin/cargo"),
        "The path to the `cargo` binary for nightly does not have the expected structure: {:?}",
        path
    );
    Ok(path.parent().unwrap().parent().unwrap().to_path_buf())
}

fn _get_crate_data(
    target_directory: &Path,
    package_id_spec: &PackageIdSpecification,
) -> Result<rustdoc_types::Crate, anyhow::Error> {
    // TODO: check that we have the nightly toolchain available beforehand in order to return
    // a good error.
    let mut cmd = std::process::Command::new("cargo");
    cmd.arg("+nightly")
        .arg("rustdoc")
        .arg("-q")
        .arg("-p")
        .arg(package_id_spec.to_string())
        .arg("--lib")
        .arg("--")
        .arg("--document-private-items")
        .arg("-Zunstable-options")
        .arg("-wjson");

    let status = cmd
        .status()
        .with_context(|| format!("Failed to run `cargo rustdoc`.\n{:?}", cmd))?;

    if !status.success() {
        anyhow::bail!(
            "An invocation of `cargo rustdoc` exited with non-zero status code.\n{:?}",
            cmd
        );
    }

    let json_path = target_directory
        .join("doc")
        .join(format!("{}.json", &package_id_spec.name));

    let json = fs_err::read_to_string(json_path).with_context(|| {
        format!(
            "Failed to read the output of a `cargo rustdoc` invocation.\n{:?}",
            cmd
        )
    })?;
    let krate = serde_json::from_str::<rustdoc_types::Crate>(&json).with_context(|| {
        format!(
            "Failed to deserialize the output of a `cargo rustdoc` invocation.\n{:?}",
            cmd
        )
    })?;
    Ok(krate)
}
