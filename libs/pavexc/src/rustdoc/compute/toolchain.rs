use anyhow::Context;
use once_cell::sync::OnceCell;
use rustdoc_types::ItemKind;
use std::path::PathBuf;

use crate::rustdoc::compute::format::check_format;

#[tracing::instrument(
    skip_all,
    fields(
        crate.name = name,
    )
)]
pub(crate) fn get_toolchain_crate_docs(
    name: &str,
    toolchain_name: &str,
) -> Result<rustdoc_types::Crate, anyhow::Error> {
    let root_folder = get_json_docs_root_folder_via_rustup(toolchain_name)?;
    let json_path = root_folder.join(format!("{}.json", name));
    let json = fs_err::read_to_string(json_path)
        .with_context(|| format!("Failed to retrieve the JSON docs for {}", name))?;
    let mut krate = match serde_json::from_str::<rustdoc_types::Crate>(&json) {
        Ok(krate) => krate,
        Err(e) => {
            return if let Err(format_err) = check_format(std::io::Cursor::new(json)) {
                Err(format_err).with_context(|| {
                    format!(
                        "The JSON docs for {name} are not in the expected format. Are you using the right version of the `nightly` toolchain, `{}`, to generate the JSON docs?",
                        crate::DEFAULT_DOCS_TOOLCHAIN
                    )
                })
            } else {
                Err(e).with_context(|| format!("Failed to deserialize the JSON docs for {}", name))
            };
        }
    };

    // Primitives, if using their fully qualified names, must be imported as `std::primitive::*`.
    // Unfortunately, that `primitive` module doesn't exist in the JSON docs, so we have to
    // manually add it.
    if name == "std" || name == "core" {
        krate.paths.values_mut().for_each(|summary| {
            if summary.kind == ItemKind::Primitive {
                summary.path.insert(1, "primitive".into());
            }
        })
    }

    Ok(krate)
}

fn get_json_docs_root_folder_via_rustup(toolchain_name: &str) -> Result<PathBuf, anyhow::Error> {
    let toolchain_root = get_toolchain_root_folder_via_rustup(toolchain_name)?;
    Ok(toolchain_root.join("share/doc/rust/json"))
}

/// In order to determine where all the components attached to a toolchain are stored,
/// we ask `rustup` to tell us the location of the `cargo` binary for that toolchain.
///
/// Experiments seem to suggest that the path to the `cargo` binary is always structured as
/// `<toolchain root folder>/bin/cargo`. Therefore we compute `<toolchain root folder>` by chopping
/// off the final two components of the path returned by `rustup`.
fn get_toolchain_root_folder_via_rustup(name: &str) -> Result<PathBuf, anyhow::Error> {
    let cargo_path = get_cargo_via_rustup(name)?;
    debug_assert!(
        cargo_path.ends_with("bin/cargo"),
        "The path to the `cargo` binary for `{name}` doesn't have the expected structure: {cargo_path:?}"
    );
    Ok(cargo_path.parent().unwrap().parent().unwrap().to_path_buf())
}

/// The path to the `cargo` binary used by the toolchain we rely on to build JSON docs.
static DOCS_TOOLCHAIN_CARGO: OnceCell<PathBuf> = OnceCell::new();

pub(super) fn get_cargo_via_rustup(toolchain_name: &str) -> Result<PathBuf, anyhow::Error> {
    fn compute_cargo_via_rustup(toolchain_name: &str) -> Result<PathBuf, anyhow::Error> {
        let mut cmd = std::process::Command::new("rustup");
        cmd.arg("which")
            .arg("--toolchain")
            .arg(toolchain_name)
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
        Ok(PathBuf::from(path))
    }

    DOCS_TOOLCHAIN_CARGO
        .get_or_try_init(|| compute_cargo_via_rustup(toolchain_name))
        .map(ToOwned::to_owned)
}
