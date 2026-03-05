use core::str;
use std::{collections::BTreeSet, path::PathBuf};

use anyhow::Context;
use camino::Utf8Path;

/// Checksum the contents of the crate at the given path in order
/// to verify that the contents have not changed.
///
/// There are two steps:
///
/// 1. Determine which files are in scope.
/// 2. Calculate the checksum of everything that was discovered, including the file names.
#[tracing::instrument("Checksum crate files", level = tracing::Level::DEBUG)]
pub fn checksum_crate(root_path: &Utf8Path) -> Result<u64, anyhow::Error> {
    let paths = get_file_paths(root_path)?;

    let mut hasher = xxhash_rust::xxh64::Xxh64::new(24);
    for path in paths {
        let contents = std::fs::read(&path)
            .with_context(|| format!("Failed to read file at `{}`", path.display()))?;
        hasher.update(&contents);
    }
    Ok(hasher.digest())
}

/// Resolves the files included in the Cargo package.
///
/// `root_dir` is the path to the directory that contains the `Cargo.toml` file.
///
/// The "canonical" way of determining the files included in the package is to run `cargo package --list`,
/// but it appears to be orders of magnitude slower than just doing "the work" ourselves.
fn get_file_paths(root_dir: &Utf8Path) -> Result<BTreeSet<PathBuf>, anyhow::Error> {
    #[derive(serde::Deserialize)]
    struct CargoManifest {
        package: Package,
    }

    #[derive(serde::Deserialize)]
    struct Package {
        include: Option<Vec<String>>,
        exclude: Option<Vec<String>>,
    }

    // Read and parse Cargo.toml
    let root_dir = root_dir
        .canonicalize()
        .context("Failed to canonicalize the path to the root directory")?;
    let toml_content = fs_err::read_to_string(root_dir.join("Cargo.toml"))?;
    let manifest: CargoManifest = toml::from_str(&toml_content)?;

    // Default inclusions
    let default_include = vec![
        "src/**".to_string(),
        "Cargo.toml".to_string(),
        // A few other files would be included (e.g. README),
        // but we don't care about them for the purpose of generating
        // the JSON docs of the crate.
    ];

    let CargoManifest {
        package: Package { include, exclude },
    } = manifest;

    // Determine include and exclude patterns
    let include_patterns = include.unwrap_or(default_include);
    let exclude_patterns = exclude.unwrap_or_default();

    let patterns: Vec<_> = include_patterns
        .into_iter()
        .chain(exclude_patterns.into_iter().map(|p| format!("!{p}")))
        .collect();

    let glob_walker = globwalk::GlobWalkerBuilder::from_patterns(&root_dir, &patterns).build()?;

    let included_files: BTreeSet<PathBuf> = glob_walker
        .into_iter()
        .filter_map(|entry| {
            let Ok(entry) = entry else {
                return None;
            };
            if !entry.file_type().is_file() {
                return None;
            }
            Some(entry.into_path())
        })
        .collect();
    Ok(included_files)
}
