use crate::locator::PavexLocator;
use anyhow::Context;
use cargo_like_utils::flock::{FileLock, Filesystem};
use cargo_like_utils::shell::Shell;
use std::io::{Read, Write};

/// The current "state" of Pavex on this machine.
///
/// It determines, in particular, what toolchain is currently active.
pub struct State {
    filesystem: Filesystem,
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
struct StateInner {
    toolchain: semver::Version,
}

impl State {
    const STATE_FILENAME: &'static str = "state.toml";

    /// Given the system home directory, create a new `State`.
    pub fn new(locator: &PavexLocator) -> Self {
        let filesystem = Filesystem::new(locator.root_dir().to_owned());
        Self { filesystem }
    }

    /// Get the current toolchain.
    ///
    /// If the toolchain is not set, it returns the toolchain that matches the current version of the CLI.
    pub fn get_current_toolchain(
        &self,
        shell: &mut Shell,
    ) -> Result<semver::Version, anyhow::Error> {
        let (_, current_state) = self.immutable_read(shell)?;
        match current_state {
            Some(current_state) => Ok(current_state.toolchain),
            None => {
                // We default to the toolchain that matches the current version of the CLI.
                let cli_version = semver::Version::parse(env!("CARGO_PKG_VERSION"))
                    .context("Failed to parse the current version of the CLI.")?;
                Ok(cli_version)
            }
        }
    }

    /// Update the current toolchain to the specified one.  
    ///
    /// It doesn't take care of installing the toolchain if it's not installed!
    pub fn set_current_toolchain(
        &self,
        shell: &mut Shell,
        toolchain: semver::Version,
    ) -> Result<(), anyhow::Error> {
        let (mut locked_file, current_state) = self.read_for_update(shell)?;
        let updated_state = match current_state {
            Some(current_state) if current_state.toolchain == toolchain => {
                // No need to do anything.
                return Ok(());
            }
            Some(mut current_state) => {
                current_state.toolchain = toolchain;
                current_state
            }
            None => StateInner { toolchain },
        };
        let updated_state = toml::to_string_pretty(&updated_state)
            .context("Failed to serialize the updated toolchain state in TOML format.")?;
        locked_file
            .write_all(updated_state.as_bytes())
            .context(format!(
                "Failed to write the updated toolchain state to {}.",
                Self::STATE_FILENAME
            ))?;
        Ok(())
    }

    /// Lock the state file for reading+writing and return the current state alongside the lock.
    /// If the state file doesn't exist, it returns `None` as the current state.
    ///
    /// Use this when you need to read the current state and update it atomically.
    fn read_for_update(
        &self,
        shell: &mut Shell,
    ) -> Result<(FileLock, Option<StateInner>), anyhow::Error> {
        let locked_file = self.filesystem.open_rw_exclusive_create(
            Self::STATE_FILENAME,
            shell,
            "toolchain state",
        )?;
        self._read(locked_file)
    }

    /// Lock the state file for reading and return the current state alongside the lock.
    /// If the state file doesn't exist, it returns `None` as the current state.
    ///
    /// Use this when you need to read the current state without updating it later.
    fn immutable_read(
        &self,
        shell: &mut Shell,
    ) -> Result<(FileLock, Option<StateInner>), anyhow::Error> {
        let locked_file = self.filesystem.open_ro_shared_create(
            Self::STATE_FILENAME,
            shell,
            "toolchain state",
        )?;
        self._read(locked_file)
    }

    fn _read(
        &self,
        mut locked_file: FileLock,
    ) -> Result<(FileLock, Option<StateInner>), anyhow::Error> {
        let mut contents = String::new();
        locked_file.read_to_string(&mut contents)?;

        if contents.is_empty() {
            return Ok((locked_file, None));
        } else {
            let contents = toml::from_str(&contents).with_context(|| {
                format!(
                    "Failed to parse the toolchain state file, {}.",
                    Self::STATE_FILENAME
                )
            })?;
            Ok((locked_file, Some(contents)))
        }
    }
}
