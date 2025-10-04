use crate::cli_kind::CliKind;
use anyhow::Context;
use guppy::Version;
use std::io::Read;
use std::path::Path;

static TAR_XZ: &str = ".tar.xz";
static ZIP: &str = ".zip";

/// Try downloading a prebuilt binary for the current host triple to the specified destination path.
pub fn download_prebuilt(
    destination: &Path,
    kind: CliKind,
    version: &Version,
) -> Result<(), DownloadPrebuiltError> {
    let host_triple = get_host_triple()?;
    let archive_suffix = match host_triple.as_str() {
        "x86_64-unknown-linux-gnu" | "x86_64-apple-darwin" | "aarch64-apple-darwin" => TAR_XZ,
        "x86_64-pc-windows-msvc" => ZIP,
        _ => {
            return Err(DownloadPrebuiltError::UnsupportedHostTriple(
                UnsupportedHostTriple {
                    triple: host_triple,
                },
            ));
        }
    };
    let download_url = format!(
        "https://github.com/LukeMathWalker/pavex/releases/download/{version}/{}-{host_triple}{archive_suffix}",
        kind.package_name()
    );
    let err_msg = "Failed to download prebuilt binary from GitHub";
    let response = ureq::get(&download_url).call().context(err_msg)?;
    if !response.status().is_success() {
        return Err(
            anyhow::anyhow!("GitHub returned a {} status code", response.status())
                .context(err_msg)
                .into(),
        );
    }
    let mut bytes = Vec::new();
    response
        .into_body()
        .into_reader()
        .read_to_end(&mut bytes)
        .context(err_msg)?;

    let expected_filename = kind.binary_filename();
    extract_binary(&download_url, &expected_filename, bytes, destination)
        .context("Failed to unpack prebuilt binary")?;

    Ok(())
}

/// Extracts the `pavexc` binary from the downloaded archive and writes it to the specified
/// destination path.
fn extract_binary(
    source_url: &str,
    expected_filename: &str,
    bytes: Vec<u8>,
    destination: &Path,
) -> Result<(), anyhow::Error> {
    if source_url.ends_with(ZIP) {
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes))?;
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let Some(file_path) = file.enclosed_name() else {
                continue;
            };
            let Some(file_name) = file_path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if file_name == expected_filename {
                let mut outfile = std::fs::File::create(destination)?;
                std::io::copy(&mut file, &mut outfile)?;
                return Ok(());
            }
        }
    } else if source_url.ends_with(TAR_XZ) {
        let decoder = xz2::bufread::XzDecoder::new(bytes.as_slice());
        let mut archive = tar::Archive::new(decoder);
        let tempdir = tempfile::tempdir()?;
        archive.unpack(tempdir.path())?;
        let mut visit_queue = vec![tempdir.path().to_owned()];
        while let Some(directory) = visit_queue.pop() {
            for entry in std::fs::read_dir(directory)? {
                let Ok(entry) = entry else {
                    continue;
                };
                let Ok(ty_) = entry.file_type() else {
                    continue;
                };
                if ty_.is_dir() {
                    visit_queue.push(entry.path());
                    continue;
                }
                let path = entry.path();
                let Some(filename) = path.file_name() else {
                    continue;
                };
                let Some(filename) = filename.to_str() else {
                    continue;
                };
                if filename == expected_filename {
                    std::fs::copy(entry.path(), destination)?;
                    return Ok(());
                }
            }
        }
    } else {
        unimplemented!()
    }
    Err(anyhow::anyhow!(
        "Failed to find `{expected_filename}` in the downloaded archive",
    ))
}

/// Returns the host triple for the current machine.
/// E.g. `x86_64-unknown-linux-gnu`.
fn get_host_triple() -> Result<String, anyhow::Error> {
    let output = std::process::Command::new("cargo")
        .arg("-vV")
        .output()
        .context("Failed to invoke `cargo -vV` to determine the host triple")?;
    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "`cargo -vV` failed with status code {}",
            output.status
        ));
    }
    let output = String::from_utf8(output.stdout)
        .context("`cargo -vV` returned non-UTF8 output. This is unexpected and probably a bug.")?;
    let output = output
        .lines()
        .find_map(|l| l.trim().strip_prefix("host: "))
        .context("`cargo -vV` returned unexpected output")?;
    Ok(output.to_owned())
}

#[derive(Debug, thiserror::Error)]
pub enum DownloadPrebuiltError {
    #[error(transparent)]
    UnsupportedHostTriple(UnsupportedHostTriple),
    #[error("{0}")]
    UnexpectedError(#[from] anyhow::Error),
}

#[derive(Debug, thiserror::Error)]
#[error("There are no prebuilt binaries for {triple}")]
pub struct UnsupportedHostTriple {
    triple: String,
}

#[cfg(test)]
mod tests {
    use super::extract_binary;

    #[test]
    fn can_decompress_zip_archives() {
        let source_url = "https://github.com/LukeMathWalker/pavex/releases/download/0.1.72/pavexc_cli-x86_64-pc-windows-msvc.zip";
        let tempdir = tempfile::tempdir().unwrap();

        // We don't commit the ZIP archive to the repository to avoid bloating it,
        // so we need to download it on the fly.
        let bytes = {
            use std::io::Read as _;

            let mut bytes = Vec::new();
            ureq::get(source_url)
                .call()
                .expect("Failed to download ZIP archive")
                .into_body()
                .into_reader()
                .read_to_end(&mut bytes)
                .expect("Failed to read the response body");
            bytes
        };

        let filename = "pavexc.exe";
        let destination = tempdir.path().join(filename);
        extract_binary(source_url, filename, bytes, &destination).unwrap();
        assert!(destination.exists());
    }

    #[test]
    fn can_decompress_tar_xz_archives() {
        let source_url = "https://github.com/LukeMathWalker/pavex/releases/download/0.1.72/pavex_cli-x86_64-apple-darwin.tar.xz";
        let tempdir = tempfile::tempdir().unwrap();

        // We don't commit the ZIP archive to the repository to avoid bloating it,
        // so we need to download it on the fly.
        let bytes = {
            use std::io::Read as _;

            let mut bytes = Vec::new();
            ureq::get(source_url)
                .call()
                .expect("Failed to download ZIP archive")
                .into_body()
                .into_reader()
                .read_to_end(&mut bytes)
                .expect("Failed to read the response body");
            bytes
        };

        let filename = "pavex";
        let destination = tempdir.path().join(filename);
        extract_binary(source_url, filename, bytes, &destination).unwrap();
        assert!(destination.exists());
    }
}
