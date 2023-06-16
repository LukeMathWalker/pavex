use anyhow::Context;
use std::path::{Path, PathBuf};

#[tracing::instrument(
    skip_all,
    fields(
        crate.name = name,
    )
)]
pub(crate) fn get_toolchain_crate_docs(name: &str) -> Result<rustdoc_types::Crate, anyhow::Error> {
    let root_folder = get_json_docs_root_folder_via_rustup()?;
    let json_path = root_folder.join(format!("{}.json", name));
    let json = fs_err::read_to_string(json_path)
        .with_context(|| format!("Failed to retrieve the JSON docs for {}", name))?;
    serde_json::from_str::<rustdoc_types::Crate>(&json)
        .with_context(|| format!("Failed to deserialize the JSON docs for {}", name))
        .map_err(Into::into)
}

fn get_json_docs_root_folder_via_rustup() -> Result<PathBuf, anyhow::Error> {
    let nightly_toolchain = get_nightly_toolchain_root_folder_via_rustup()?;
    Ok(nightly_toolchain.join("share/doc/rust/json"))
}

/// In order to determine where all the components attached to the `nightly` toolchain are stored,
/// we ask `rustup` to tell us the location of the `cargo` binary for `nightly`.
///
/// Experiments seem to suggest that the path to the `cargo` binary is always structured as
/// `<toolchain root folder>/bin/cargo`. Therefore we compute `<toolchain root folder>` by chopping
/// off the final two components of the path returned by `rustup`.
fn get_nightly_toolchain_root_folder_via_rustup() -> Result<PathBuf, anyhow::Error> {
    let mut cmd = std::process::Command::new("rustup");
    cmd.arg("which")
        .arg("--toolchain")
        .arg("nightly")
        .arg("cargo");

    let output = cmd.output().with_context(|| {
        format!("Failed to run a `rustup` command. Is `rustup` installed?\n{cmd:?}")
    })?;

    if !output.status.success() {
        anyhow::bail!(
            "An invocation of `rustup` exited with non-zero status code.\n{:?}",
            cmd
        );
    }
    let path = std::str::from_utf8(&output.stdout)
        .with_context(|| {
            format!("An invocation of `rustup` returned non-UTF8 data as output.\n{cmd:?}")
        })?
        .trim();
    let path = Path::new(path);
    debug_assert!(
        path.ends_with("bin/cargo"),
        "The path to the `cargo` binary for nightly doesn't have the expected structure: {path:?}"
    );
    Ok(path.parent().unwrap().parent().unwrap().to_path_buf())
}
