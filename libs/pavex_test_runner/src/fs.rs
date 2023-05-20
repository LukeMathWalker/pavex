use std::path::Path;
use sha2::Digest;
use std::io::{Read, Write};

/// Only persist the content if it differs from the one already on disk.
///
/// It if the file does not exist, it will be created.
/// 
/// This is useful to avoid unnecessary rebuilds, since `cargo` takes into account
/// the modification time of the files when determining if they have changed or not.
pub(crate) fn persist_if_changed(path: &Path, content: &[u8]) -> Result<(), anyhow::Error> {
    if let Ok(file_checksum) = compute_file_checksum(path) {
        let buffer_checksum = compute_buffer_checksum(content);
        if file_checksum == buffer_checksum {
            return Ok(());
        }
    }
    let mut file = fs_err::OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(path)?;
    file.write_all(content)?;
    Ok(())
}

pub(crate) fn copy_if_changed(from: &Path, to: &Path) -> Result<(), anyhow::Error> {
    if let Ok(from_checksum) = compute_file_checksum(from) {
        if let Ok(to_checksum) = compute_file_checksum(to) {
            if from_checksum == to_checksum {
                return Ok(());
            }
        }
    }
    fs_err::copy(from, to)?;
    Ok(())
}

/// Compute the checksum of a file, if it exists.
fn compute_file_checksum(path: &Path) -> std::io::Result<String> {
    let mut hasher = sha2::Sha256::new();

    let file = fs_err::File::open(path)?;
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
fn compute_buffer_checksum(buffer: &[u8]) -> String {
    let mut hasher = sha2::Sha256::new();
    hasher.update(buffer);
    let result = hasher.finalize();
    format!("{:x}", result)
}
