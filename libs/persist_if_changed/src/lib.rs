//! A tiny utility library to avoid touching the filesystem if the content has not changed.
//!
//! This is useful to avoid triggering unnecessary rebuilds in systems that look at
//! the modification time (`mtime`) as part of their file fingerprint (e.g. `cargo`).
use sha2::Digest;
use std::{
    io::{Read, Write},
    path::Path,
};

/// Only persist the content if it differs from the one already on disk.
///
/// It if the file does not exist, it will be created.
///
/// This is useful to avoid unnecessary rebuilds, since `cargo` takes into account
/// the modification time of the files when determining if they have changed or not.
#[tracing::instrument(skip_all, level=tracing::Level::TRACE)]
pub fn persist_if_changed(path: &Path, content: &[u8]) -> Result<(), anyhow::Error> {
    let has_changed = has_changed_file2buffer(path, content).unwrap_or(true);
    if !has_changed {
        return Ok(());
    }
    let mut file = fs_err::OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(path)?;
    file.write_all(content)?;
    Ok(())
}

/// Only copy the file if its contents differ from the contents stored at
/// the destination path.
#[tracing::instrument(skip_all, level=tracing::Level::TRACE)]
pub fn copy_if_changed(from: &Path, to: &Path) -> Result<(), anyhow::Error> {
    let has_changed = has_changed_file2file(from, to).unwrap_or(true);
    if !has_changed {
        return Ok(());
    }
    fs_err::copy(from, to)?;
    Ok(())
}

/// Returns `true` if the file contents are different, `false` otherwise.
///
/// It returns an error if we could not determine the outcome due to a
/// failure in any of the intermediate operations (e.g. there is no file
/// at the destination path).
fn has_changed_file2file(from: &Path, to: &Path) -> Result<bool, anyhow::Error> {
    let from_file = fs_err::File::open(from)?;
    let to_file = fs_err::File::open(to)?;

    // Cheaper check first: if the file size is not the same,
    // we can skip computing the checksum.
    let from_metadata = from_file.metadata()?;
    let to_metadata = to_file.metadata()?;
    if from_metadata.len() != to_metadata.len() {
        return Ok(true);
    }

    let from_checksum = compute_file_checksum(from_file)?;
    let to_checksum = compute_file_checksum(to_file)?;
    Ok(from_checksum != to_checksum)
}

/// Returns `true` if the file contents are different from the buffer, `false` otherwise.
///
/// It returns an error if we could not determine the outcome due to a
/// failure in any of the intermediate operations (e.g. the file doesn't exist).
fn has_changed_file2buffer(path: &Path, contents: &[u8]) -> Result<bool, anyhow::Error> {
    let file = fs_err::File::open(path)?;
    // Cheaper check first: if the file size is not the same,
    // we can skip computing the checksum.
    let metadata = file.metadata()?;
    if metadata.len() != contents.len() as u64 {
        return Ok(true);
    }
    let file_checksum = compute_file_checksum(file)?;
    let buffer_checksum = compute_buffer_checksum(contents);
    Ok(file_checksum != buffer_checksum)
}

/// Compute the checksum of a file, if it exists.
#[tracing::instrument(skip_all, level=tracing::Level::TRACE)]
fn compute_file_checksum(file: fs_err::File) -> std::io::Result<String> {
    let mut hasher = sha2::Sha256::new();

    let mut reader = std::io::BufReader::new(file);
    let mut buffer = [0; 8192]; // Buffer size (adjust as needed)

    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}

/// Compute the checksum of an in-memory bytes buffer.
#[tracing::instrument(skip_all, level=tracing::Level::TRACE)]
fn compute_buffer_checksum(buffer: &[u8]) -> String {
    let mut hasher = sha2::Sha256::new();
    hasher.update(buffer);
    let result = hasher.finalize();
    format!("{:x}", result)
}
