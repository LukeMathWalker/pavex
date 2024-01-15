use crate::cargo_install::{cargo_install, GitSourceRevision, Source};
use crate::pavexc::prebuilt::download_prebuilt;
use cargo_like_utils::shell::Shell;
use guppy::graph::{ExternalSource, GitReq, PackageSource};
use guppy::Version;
use std::path::{Path, PathBuf};

/// Given the version and source for the `pavex` library crate, install the corresponding
/// `pavexc` binary crate at the specified path.
pub(super) fn install(
    shell: &mut Shell,
    pavexc_cli_path: &Path,
    version: &Version,
    package_source: &PackageSource,
) -> Result<(), InstallError> {
    match package_source {
        PackageSource::Workspace(_) => {
            if !pavexc_cli_path.exists() {
                return Err(InstallError::NoWorkspaceBinary(NoWorkspaceBinaryError {
                    pavexc_expected_path: pavexc_cli_path.to_path_buf(),
                }));
            }
        }
        PackageSource::Path(p) => {
            let workspace_root = p
                .parent()
                .expect("pavex's source path has to have a parent");
            if !pavexc_cli_path.exists() {
                return Err(InstallError::NoLocalBinary(NoLocalBinaryError {
                    pavex_source_path: p.as_std_path().to_path_buf(),
                    pavexc_expected_path: pavexc_cli_path.to_path_buf(),
                    workspace_path: workspace_root.as_std_path().to_path_buf(),
                }));
            }
        }
        PackageSource::External(_) => {
            let parsed = package_source.parse_external();
            let Some(parsed) = parsed else {
                return Err(InstallError::InvalidSource(InvalidSourceError {
                    package_source: package_source.to_string(),
                    version: version.to_owned(),
                }));
            };
            match parsed {
                ExternalSource::Registry(c) => {
                    if !package_source.is_crates_io() {
                        return Err(InstallError::UnsupportedSource(UnsupportedSourceError {
                            package_source: format!(
                                "a private registry ({})",
                                c.strip_prefix("registry+").unwrap_or(c)
                            ),
                            version: version.to_owned(),
                        }));
                    }

                    match download_prebuilt(pavexc_cli_path, version) {
                        Ok(_) => {
                            return Ok(());
                        }
                        Err(e) => {
                            let _ = shell.warn(
                                "Failed to download prebuilt `pavexc` binary. I'll try to build it from source instead.",
                            );
                            tracing::warn!(
                                error.msg = %e,
                                error.cause = ?e,
                                "Failed to download prebuilt `pavexc` binary. I'll try to build it from source instead.",
                            );
                        }
                    }

                    cargo_install(
                        Source::CratesIo {
                            version: version.to_string(),
                        },
                        "pavexc",
                        "pavexc_cli",
                        pavexc_cli_path,
                    )?;
                }
                ExternalSource::Git {
                    repository,
                    req,
                    resolved,
                } => {
                    if repository == "https://github.com/LukeMathWalker/pavex" {
                        if let GitReq::Tag(tag) = req {
                            if tag == version.to_string() {
                                match download_prebuilt(pavexc_cli_path, version) {
                                    Ok(_) => {
                                        return Ok(());
                                    }
                                    Err(e) => {
                                        let _ = shell.warn(
                                            "Failed to download prebuilt `pavexc` binary. I'll try to build it from source instead.",
                                        );
                                        tracing::warn!(
                                            error.msg = %e,
                                            error.cause = ?e,
                                            "Failed to download prebuilt `pavexc` binary. I'll try to build it from source instead.",
                                        );
                                    }
                                }
                            }
                        }
                    }
                    cargo_install(
                        Source::Git {
                            url: repository.into(),
                            rev: GitSourceRevision::Rev(resolved.into()),
                        },
                        "pavexc",
                        "pavexc_cli",
                        pavexc_cli_path,
                    )?;
                }
                _ => {
                    return Err(InstallError::UnsupportedSource(UnsupportedSourceError {
                        package_source: package_source.to_string(),
                        version: version.to_owned(),
                    }));
                }
            }
        }
    }
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum InstallError {
    #[error(transparent)]
    InvalidSource(InvalidSourceError),
    #[error(transparent)]
    UnsupportedSource(UnsupportedSourceError),
    #[error(transparent)]
    NoLocalBinary(NoLocalBinaryError),
    #[error(transparent)]
    NoWorkspaceBinary(NoWorkspaceBinaryError),
    #[error("{0}")]
    UnexpectedError(#[from] anyhow::Error),
}

#[derive(Debug, thiserror::Error)]
#[error("`pavex` can't automatically install `pavexc@{version}` from {package_source}")]
pub struct UnsupportedSourceError {
    pub(crate) package_source: String,
    pub(crate) version: Version,
}

#[derive(Debug, thiserror::Error)]
#[error("`pavex` doesn't recognise `{package_source}` as a valid source to install `pavexc@{version}` from")]
pub struct InvalidSourceError {
    pub(crate) package_source: String,
    pub(crate) version: Version,
}

#[derive(Debug, thiserror::Error)]
#[error("You are using a local version of `pavex` (source at `{pavex_source_path}`). I expect the corresponding `pavexc` binary crate to be available at `{pavexc_expected_path}`, but it's not there. Did you forget to run `cargo build --release --bin pavexc` from `{workspace_path}`?")]
pub struct NoLocalBinaryError {
    pavex_source_path: PathBuf,
    pavexc_expected_path: PathBuf,
    workspace_path: PathBuf,
}

#[derive(Debug, thiserror::Error)]
#[error("You are using a version of `pavex` from the current workspace. I expect the corresponding `pavexc` binary crate to be available at `{pavexc_expected_path}`, but it's not there. Did you forget to run `cargo build --release --bin pavexc`?")]
pub struct NoWorkspaceBinaryError {
    pavexc_expected_path: PathBuf,
}
