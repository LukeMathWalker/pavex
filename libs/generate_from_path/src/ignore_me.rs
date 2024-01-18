use std::fs::remove_file;
use std::path::{Path, PathBuf};

/// Takes the directory path and removes the files/directories specified in the
/// `.genignore` file
/// It handles all errors internally
pub fn remove_unneeded_files(
    dir: &Path,
    ignored_files: &Option<Vec<String>>,
    verbose: bool,
) -> anyhow::Result<()> {
    let mut items = vec![];
    if let Some(ignored_files) = ignored_files {
        for f in ignored_files {
            let mut p = PathBuf::new();
            p.push(dir);
            p.push(f);
            items.push(p);
        }
    }
    remove_dir_files(&items, verbose);
    Ok(())
}

pub fn remove_dir_files(files: impl IntoIterator<Item = impl Into<PathBuf>>, verbose: bool) {
    for item in files
        .into_iter()
        .map(|i| i.into() as PathBuf)
        .filter(|file| file.exists())
    {
        let ignore_message = format!("Ignoring: {}", &item.display());
        if item.is_dir() {
            fs_err::remove_dir_all(&item).unwrap();
            if verbose {
                tracing::info!("{ignore_message}");
            }
        } else if item.is_file() {
            remove_file(&item).unwrap();
            if verbose {
                tracing::info!("{ignore_message}");
            }
        } else {
            tracing::warn!(
                "The given paths are neither files nor directories! {}",
                &item.display()
            );
        }
    }
}
