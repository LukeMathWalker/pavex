use core::str;

use anyhow::Context;
use camino::{Utf8Path, Utf8PathBuf};

/// Checksum the contents of the crate at the given path in order
/// to verify that the contents have not changed.
///
/// There are two steps:
///
/// 1. Determine which files are in scope. This is done via `cargo package --list`,
///    to avoid checksumming files that are not part of the crate (e.g. its target directory).
/// 2. Calculate the checksum of everything that was discovered, including the file names.
#[tracing::instrument("Checksum crate files", level = tracing::Level::DEBUG)]
pub(super) fn checksum_crate(root_path: &Utf8Path) -> Result<u64, anyhow::Error> {
    let paths = get_paths(root_path)?;

    let mut hasher = xxhash_rust::xxh64::Xxh64::new(0);
    for path in paths {
        // Read and hash the file contents
        // We don't check if the path is a directory because `cargo package --list`
        // only lists files
        let contents =
            std::fs::read(&path).with_context(|| format!("Failed to read file at `{}`", path))?;
        hasher.update(&contents);
    }
    Ok(hasher.digest())
}

#[tracing::instrument("Get file paths via `cargo package --list`", level = tracing::Level::DEBUG)]
fn get_paths(root_path: &Utf8Path) -> Result<Vec<Utf8PathBuf>, anyhow::Error> {
    let mut command = std::process::Command::new("cargo");
    command
        .arg("package")
        .arg("--list")
        .arg("--quiet")
        .current_dir(root_path);
    let output = command
        .output()
        .with_context(|| format!("Failed to run `cargo package --list` in `{root_path}`"))?;
    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!(
            "`cargo package --list` failed. Stdout:\n{stdout}\nStderr:\n{stderr}",
        ));
    }
    let stdout =
        str::from_utf8(&output.stdout).context("`cargo package --list` returned non-UTF8 paths")?;

    let mut files = Vec::new();
    for line in stdout.lines() {
        if line == "Cargo.toml.orig" || line == ".cargo_vcs_info.json" {
            // Skip these files, they are not part of the crate
            // They are created by cargo itself when packaging
            continue;
        }
        // All paths are relative to the root
        let path = root_path.join(line);
        files.push(path);
    }
    Ok(files)
}
