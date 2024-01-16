use anyhow::Context;
use guppy::Version;
use std::io::Read;
use std::path::Path;

/// Given the version and source for the `pavex` library crate, try to download
/// a prebuilt `pavexc` binary (if it exists).
pub(super) fn download_prebuilt(
    expected_pavexc_cli_path: &Path,
    version: &Version,
) -> Result<(), DownloadPrebuiltError> {
    let host_triple = get_host_triple()?;
    let url_prefix =
        format!("https://github.com/LukeMathWalker/pavex/releases/download/{version}/pavexc_cli-{host_triple}");
    let download_url = match host_triple.as_str() {
        "x86_64-unknown-linux-gnu" | "x86_64-apple-darwin" | "aarch64-apple-darwin" => {
            format!("{url_prefix}.tar.xz")
        }
        "x86_64-pc-windows-msvc" => {
            format!("{url_prefix}.zip")
        }
        _ => {
            return Err(DownloadPrebuiltError::UnsupportedHostTriple(
                UnsupportedHostTriple {
                    triple: host_triple,
                },
            ))
        }
    };
    let err_msg = "Failed to download prebuilt binary from GitHub";
    let response = ureq::get(&download_url).call().context(err_msg)?;
    if response.status() < 200 || response.status() >= 300 {
        return Err(
            anyhow::anyhow!("GitHub returned a {} status code", response.status())
                .context(err_msg)
                .into(),
        );
    }
    let mut bytes = Vec::new();
    response
        .into_reader()
        .read_to_end(&mut bytes)
        .context(err_msg)?;

    extract_binary(&download_url, bytes, expected_pavexc_cli_path)
        .context("Failed to unpack prebuilt binary")?;

    Ok(())
}

/// Extracts the `pavexc` binary from the downloaded archive and writes it to the specified
/// destination path.
fn extract_binary(
    source_url: &str,
    bytes: Vec<u8>,
    destination: &Path,
) -> Result<(), anyhow::Error> {
    let expected_filename = destination
        .file_name()
        .expect("pavexc's destination has no filename")
        .to_str()
        .expect("pavexc's destination filename is not valid UTF-8");
    if source_url.ends_with(".zip") {
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes))?;
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            if file.name() == expected_filename {
                let mut outfile = std::fs::File::create(&destination)?;
                std::io::copy(&mut file, &mut outfile)?;
                return Ok(());
            }
        }
    } else if source_url.ends_with(".tar.xz") {
        let mut archive = tar::Archive::new(xz2::read::XzDecoder::new(bytes.as_slice()));
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
        "Failed to find the `pavexc` binary in the downloaded archive"
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
