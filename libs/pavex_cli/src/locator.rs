use semver::Version;
use sha2::Digest;
use std::path::{Path, PathBuf};

/// A unified entrypoint to locate Pavex-related files and directories
/// on a user system.
pub struct PavexLocator {
    pavex_dir: PathBuf,
}

impl PavexLocator {
    /// Create a new [`PavexLocator`] rooted at the given system home directory.
    pub fn new(system_home_dir: &Path) -> Self {
        PavexLocator {
            pavex_dir: system_home_dir.join(".pavex"),
        }
    }

    /// Look at the installed toolchains.
    pub fn toolchains(&self) -> ToolchainsLocator {
        ToolchainsLocator {
            toolchain_dir: self.pavex_dir.join("toolchains"),
        }
    }

    pub fn root_dir(&self) -> &Path {
        &self.pavex_dir
    }
}

/// A unified entrypoint to locate all toolchain-related files.
pub struct ToolchainsLocator {
    toolchain_dir: PathBuf,
}

impl ToolchainsLocator {
    pub fn root_dir(&self) -> &Path {
        &self.toolchain_dir
    }

    /// Look at the toolchains installed from a `git` source.
    pub fn git(&self) -> GitToolchainsLocator {
        GitToolchainsLocator {
            git_toolchain_dir: self.toolchain_dir.join("git"),
        }
    }

    /// Look at the toolchains installed from a `registry` source.
    pub fn registry(&self) -> RegistryToolchainsLocator {
        RegistryToolchainsLocator {
            registry_toolchain_dir: self.toolchain_dir.join("registry"),
        }
    }
}

/// A unified entrypoint to locate all toolchain-related files for `git` toolchains.
pub struct GitToolchainsLocator {
    git_toolchain_dir: PathBuf,
}

impl GitToolchainsLocator {
    pub fn root_dir(&self) -> &Path {
        &self.git_toolchain_dir
    }

    /// Look at a specific `git` toolchain.
    pub fn toolchain_dir(&self, repository: &str, revision_sha: &str) -> ToolchainLocator {
        let repository_hash = sha2::Sha256::digest(repository.as_bytes());
        // Take the first 7 hex digits of the hash
        let repository_hash = format!("{:x}", repository_hash)
            .chars()
            .take(7)
            .collect::<String>();
        // Take the first 7 hex digits of the hash, i.e. git's short commit SHA
        let revision_sha = revision_sha.chars().take(7).collect::<String>();
        let toolchain_dir = self
            .git_toolchain_dir
            .join(repository_hash)
            .join(revision_sha);
        ToolchainLocator { toolchain_dir }
    }
}

/// A unified entrypoint to locate all toolchain-related files for toolchains installed from
/// a `registry` source.
pub struct RegistryToolchainsLocator {
    registry_toolchain_dir: PathBuf,
}

impl RegistryToolchainsLocator {
    pub fn root_dir(&self) -> &Path {
        &self.registry_toolchain_dir
    }

    /// Look at a specific `registry` toolchain.
    pub fn toolchain_dir(&self, registry: &str, version: &Version) -> ToolchainLocator {
        let registry_hash = sha2::Sha256::digest(registry.as_bytes());
        // Take the first 7 hex digits of the hash
        let registry_hash = format!("{:x}", registry_hash)
            .chars()
            .take(7)
            .collect::<String>();
        // Take the first 7 hex digits of the hash, i.e. git's short commit SHA
        let version = version.to_string().chars().take(7).collect::<String>();
        let toolchain_dir = self
            .registry_toolchain_dir
            .join(registry_hash)
            .join(version);
        ToolchainLocator { toolchain_dir }
    }
}

/// A unified entrypoint for all files related to a single toolchain.
pub struct ToolchainLocator {
    toolchain_dir: PathBuf,
}

impl ToolchainLocator {
    pub fn root_dir(&self) -> &Path {
        &self.toolchain_dir
    }

    /// Path to the `pavexc` binary for this toolchain.
    pub fn pavexc(&self) -> PathBuf {
        self.toolchain_dir.join("pavexc")
    }
}
