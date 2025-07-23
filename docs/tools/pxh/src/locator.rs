use std::path::Path;

use camino::Utf8PathBuf;
use globwalk::GlobWalkerBuilder;

/// Find the manifests of the examples that are in scope for this invocation.
///
/// It returns all manifests from the current working directory and its subdirectories.
pub fn find_examples_in_scope(cwd: &Path) -> Result<Vec<Utf8PathBuf>, anyhow::Error> {
    find_manifests_in_scope(cwd, "example")
}

/// Find the manifests of the tutorials that are in scope for this invocation.
///
/// It returns all manifests from the current working directory and its subdirectories.
pub fn find_tutorials_in_scope(cwd: &Path) -> Result<Vec<Utf8PathBuf>, anyhow::Error> {
    find_manifests_in_scope(cwd, "tutorial")
}

/// Find the YAML manifests with a given name that are in scope for this invocation.
///
/// It returns all manifests from the current working directory and its subdirectories.
pub fn find_manifests_in_scope(
    cwd: &Path,
    file_name: &str,
) -> Result<Vec<Utf8PathBuf>, anyhow::Error> {
    let manifests = GlobWalkerBuilder::from_patterns(cwd, &[format!("**/{file_name}.yml")])
        .build()
        .expect("Failed to build glob walker")
        .filter_map(|entry| {
            let Some(entry) = entry.ok() else { return None };
            entry.into_path().try_into().ok()
        })
        .collect();
    Ok(manifests)
}
