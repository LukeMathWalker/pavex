use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

pub fn main() -> Result<()> {
    // Compute checksum of pavexc_rustdoc_cache and its local dependencies.
    // This checksum is used as part of the cache fingerprint to ensure
    // the cache invalidates when the caching logic or serialized types change.
    let base_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let cache_crate_path = base_path.join("pavexc_rustdoc_cache");

    // Find all local crates that pavexc_rustdoc_cache depends on (transitively)
    let crates_to_checksum = collect_local_dependencies(&cache_crate_path)?;

    let mut combined_hasher = xxhash_rust::xxh64::Xxh64::new(24);
    for crate_path in &crates_to_checksum {
        let checksum = checksum_directory(crate_path)?;
        combined_hasher.update(&checksum.to_le_bytes());

        // Rerun if any of these crates change
        println!("cargo::rerun-if-changed={}/src", crate_path.display());
        println!("cargo::rerun-if-changed={}/Cargo.toml", crate_path.display());
    }

    let checksum = combined_hasher.digest();
    println!("cargo::rustc-env=RUSTDOC_CACHE_SOURCE_HASH={checksum:x}");

    Ok(())
}

/// Collect all local path dependencies of a crate, including the crate itself.
/// This is done recursively to capture transitive local dependencies.
fn collect_local_dependencies(crate_path: &Path) -> Result<BTreeSet<PathBuf>> {
    let mut visited = BTreeSet::new();
    let mut to_visit = vec![crate_path.to_path_buf()];

    while let Some(current) = to_visit.pop() {
        let canonical = current
            .canonicalize()
            .with_context(|| format!("Failed to canonicalize path: {}", current.display()))?;

        if !visited.insert(canonical.clone()) {
            continue;
        }

        // Parse Cargo.toml to find path dependencies
        let cargo_toml_path = canonical.join("Cargo.toml");
        let cargo_toml_content = std::fs::read_to_string(&cargo_toml_path)
            .with_context(|| format!("Failed to read {}", cargo_toml_path.display()))?;

        let cargo_toml: toml::Table = toml::from_str(&cargo_toml_content)
            .with_context(|| format!("Failed to parse {}", cargo_toml_path.display()))?;

        // Check [dependencies] section for path dependencies
        if let Some(toml::Value::Table(deps)) = cargo_toml.get("dependencies") {
            for (_name, value) in deps {
                if let Some(path) = value.get("path").and_then(|p| p.as_str()) {
                    let dep_path = canonical.join(path);
                    if dep_path.exists() {
                        to_visit.push(dep_path);
                    }
                }
            }
        }
    }

    Ok(visited)
}

/// Checksum the contents of a crate directory.
fn checksum_directory(root_path: &Path) -> Result<u64> {
    let paths = get_file_paths(root_path)?;

    let mut hasher = xxhash_rust::xxh64::Xxh64::new(24);
    for path in paths {
        let contents = std::fs::read(&path)
            .with_context(|| format!("Failed to read file at `{}`", path.display()))?;
        hasher.update(&contents);
        // Include the file path in the hash to detect renames
        if let Ok(relative) = path.strip_prefix(root_path) {
            hasher.update(relative.to_string_lossy().as_bytes());
        }
    }
    Ok(hasher.digest())
}

/// Get all source files in a crate directory.
fn get_file_paths(root_dir: &Path) -> Result<BTreeSet<PathBuf>> {
    let root_dir = root_dir
        .canonicalize()
        .context("Failed to canonicalize the path to the root directory")?;

    let patterns = vec!["src/**/*.rs", "Cargo.toml"];

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
