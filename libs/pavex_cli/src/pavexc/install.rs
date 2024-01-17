use crate::cargo_install::{cargo_install, GitSourceRevision, Source};
use crate::prebuilt::{download_prebuilt, PrebuiltBinaryKind};
use cargo_like_utils::shell::Shell;
use guppy::graph::PackageSource;
use guppy::Version;
use std::path::{Path, PathBuf};

pub enum InstallSource {
    /// Install the binary from the current workspace.
    Workspace,
    /// Install the binary from the specified path.
    Path(PathBuf),
    /// Install the binary from the specified external source.
    External(ExternalSource),
}

pub enum ExternalSource {
    Registry {
        url: String,
    },
    Git {
        repository: String,
        req: GitReq,
        resolved: Option<String>,
    },
}

pub enum GitReq {
    Tag(String),
    Rev(String),
    Branch(String),
}

impl<'a> TryFrom<PackageSource<'a>> for InstallSource {
    type Error = InstallError;

    fn try_from(value: PackageSource<'a>) -> Result<Self, Self::Error> {
        match value {
            PackageSource::Workspace(_) => Ok(InstallSource::Workspace),
            PackageSource::Path(p) => Ok(InstallSource::Path(p.as_std_path().to_path_buf())),
            PackageSource::External(s) => {
                let parsed = value.parse_external();
                let Some(parsed) = parsed else {
                    return Err(InstallError::InvalidSource(InvalidSourceError {
                        package_source: s.to_string(),
                    }));
                };
                match parsed {
                    guppy::graph::ExternalSource::Registry(c) => {
                        Ok(InstallSource::External(ExternalSource::Registry {
                            url: c.strip_prefix("registry+").unwrap_or(c).into(),
                        }))
                    }
                    guppy::graph::ExternalSource::Git {
                        repository,
                        req,
                        resolved,
                    } => Ok(InstallSource::External(ExternalSource::Git {
                        repository: repository.into(),
                        req: match req {
                            guppy::graph::GitReq::Branch(b) => GitReq::Branch(b.into()),
                            guppy::graph::GitReq::Tag(t) => GitReq::Tag(t.into()),
                            guppy::graph::GitReq::Rev(r) => GitReq::Rev(r.into()),
                            guppy::graph::GitReq::Default => GitReq::Branch("main".to_string()),
                            _ => {
                                return Err(InstallError::UnsupportedSource(
                                    UnsupportedSourceError {
                                        package_source: format!("an unknown `git` source ({s})",),
                                    },
                                ))
                            }
                        },
                        resolved: Some(resolved.into()),
                    })),
                    _ => Err(InstallError::UnsupportedSource(UnsupportedSourceError {
                        package_source: s.to_string(),
                    })),
                }
            }
        }
    }
}

/// Given the version and source for the `pavex` library crate, install the corresponding
/// `pavexc` binary crate at the specified path.
pub(super) fn install(
    shell: &mut Shell,
    pavexc_cli_path: &Path,
    version: &Version,
    install_source: &InstallSource,
) -> Result<(), InstallError> {
    let (try_prebuilt, install_source) = match install_source {
        InstallSource::Workspace => {
            if !pavexc_cli_path.exists() {
                return Err(InstallError::NoWorkspaceBinary(NoWorkspaceBinaryError {
                    pavexc_expected_path: pavexc_cli_path.to_path_buf(),
                }));
            } else {
                return Ok(());
            }
        }
        InstallSource::Path(p) => {
            let workspace_root = p
                .parent()
                .expect("pavex's source path has to have a parent");
            if !pavexc_cli_path.exists() {
                return Err(InstallError::NoLocalBinary(NoLocalBinaryError {
                    pavex_source_path: p.to_owned(),
                    pavexc_expected_path: pavexc_cli_path.to_path_buf(),
                    workspace_path: workspace_root.to_path_buf(),
                }));
            } else {
                return Ok(());
            }
        }
        InstallSource::External(ExternalSource::Registry { url }) => {
            if url != guppy::graph::ExternalSource::CRATES_IO_URL {
                return Err(InstallError::UnsupportedSource(UnsupportedSourceError {
                    package_source: format!("a private registry ({})", url),
                }));
            }
            (
                true,
                Source::CratesIo {
                    version: version.to_string(),
                },
            )
        }
        InstallSource::External(ExternalSource::Git {
            repository,
            req,
            resolved,
        }) => {
            let mut try_prebuilt = false;
            if repository == "https://github.com/LukeMathWalker/pavex" {
                if let GitReq::Tag(tag) = req {
                    if tag == version.to_string().as_str() {
                        try_prebuilt = true;
                    }
                }
            }
            let git_source = match resolved {
                None => match req {
                    GitReq::Tag(tag) => GitSourceRevision::Tag(tag.into()),
                    GitReq::Rev(rev) => GitSourceRevision::Rev(rev.into()),
                    GitReq::Branch(branch) => GitSourceRevision::Branch(branch.into()),
                },
                Some(r) => GitSourceRevision::Rev(r.into()),
            };
            (
                try_prebuilt,
                Source::Git {
                    url: repository.into(),
                    rev: git_source,
                },
            )
        }
    };

    if try_prebuilt {
        let _ = shell.status("Downloading", format!("prebuilt `pavexc@{version}` binary"));
        match download_prebuilt(pavexc_cli_path, PrebuiltBinaryKind::Pavexc, version) {
            Ok(_) => {
                let _ = shell.status("Downloaded", format!("prebuilt `pavexc@{version}` binary"));
                return Ok(());
            }
            Err(e) => {
                let _ =
                    shell.warn("Download failed: {e}.\nI'll try compiling from source instead.");
                tracing::warn!(
                    error.msg = %e,
                    error.cause = ?e,
                    "Failed to download prebuilt `pavexc` binary. I'll try to build it from source instead.",
                );
            }
        }
    }

    let _ = shell.status("Compiling", format!("`pavexc@{version}` from source"));
    cargo_install(install_source, "pavexc", "pavexc_cli", pavexc_cli_path)?;
    let _ = shell.status("Compiled", format!("`pavexc@{version}` from source"));
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
#[error("`pavex` can't automatically install `pavexc` from {package_source}")]
pub struct UnsupportedSourceError {
    pub(crate) package_source: String,
}

#[derive(Debug, thiserror::Error)]
#[error("`pavex` doesn't recognise `{package_source}` as a valid source to install `pavexc` from")]
pub struct InvalidSourceError {
    pub(crate) package_source: String,
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
