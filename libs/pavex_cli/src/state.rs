use crate::env::version;
use crate::locator::PavexLocator;
use anyhow::Context;
use cargo_like_utils::flock::{FileLock, Filesystem};
use cargo_like_utils::shell::Shell;
use secrecy::{ExposeSecret, SecretString};
use std::io::{Read, Write};

/// The current "state" of Pavex on this machine.
///
/// It determines, in particular, what toolchain is currently active.
pub struct State {
    filesystem: Filesystem,
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Default)]
struct StateInner {
    /// The toolchain that's currently active.
    #[serde(skip_serializing_if = "Option::is_none")]
    toolchain: Option<semver::Version>,
    /// The activation key associated with this installation of Pavex.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(serialize_with = "serialize_activation_key")]
    activation_key: Option<SecretString>,
}

fn serialize_activation_key<S>(
    activation_key: &Option<SecretString>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match activation_key {
        Some(activation_key) => serializer.serialize_some(activation_key.expose_secret()),
        None => serializer.serialize_none(),
    }
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
    ) -> Result<semver::Version, StateReadError> {
        let (_, current_state) = self.immutable_read(shell)?;
        let toolchain = current_state.and_then(|s| s.toolchain);
        match toolchain {
            Some(toolchain) => Ok(toolchain),
            None => {
                // We default to the toolchain that matches the current version of the CLI.
                Ok(version())
            }
        }
    }

    /// Get the activation key associated with this installation, if there is one.
    pub fn get_activation_key(
        &self,
        shell: &mut Shell,
    ) -> Result<Option<SecretString>, StateReadError> {
        let (_, current_state) = self.immutable_read(shell)?;
        Ok(current_state.and_then(|s| s.activation_key))
    }

    /// Set the activation key associated with this installation.
    pub fn set_activation_key(
        &self,
        shell: &mut Shell,
        activation_key: SecretString,
    ) -> Result<(), anyhow::Error> {
        let (mut locked_file, state) = self.read_for_update(shell)?;
        let mut state = state.unwrap_or_default();
        state.activation_key = Some(activation_key);
        let state = toml::to_string_pretty(&state)
            .context("Failed to serialize Pavex's updated state in TOML format.")?;
        locked_file.write_all(state.as_bytes()).context(format!(
            "Failed to write Pavex's updated state to {}.",
            Self::STATE_FILENAME
        ))?;
        Ok(())
    }

    /// Update the current toolchain to the specified one.  
    ///
    /// It doesn't take care of installing the toolchain if it's not installed!
    pub fn set_current_toolchain(
        &self,
        shell: &mut Shell,
        toolchain: semver::Version,
    ) -> Result<(), anyhow::Error> {
        let (mut locked_file, state) = self.read_for_update(shell)?;
        let mut state = state.unwrap_or_default();
        if state.toolchain.as_ref() == Some(&toolchain) {
            // No need to do anything.
            return Ok(());
        } else {
            state.toolchain = Some(toolchain);
        }
        let state = toml::to_string_pretty(&state)
            .context("Failed to serialize Pavex's updated state in TOML format.")?;
        locked_file.write_all(state.as_bytes()).context(format!(
            "Failed to write Pavex's updated state to {}.",
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
    ) -> Result<(FileLock, Option<StateInner>), StateReadError> {
        let locked_file = self
            .filesystem
            .open_rw_exclusive_create(Self::STATE_FILENAME, shell, "Pavex's state file")
            .map_err(AcquireLockError)?;
        self._read(locked_file)
    }

    /// Lock the state file for reading and return the current state alongside the lock.
    /// If the state file doesn't exist, it returns `None` as the current state.
    ///
    /// Use this when you need to read the current state without updating it later.
    fn immutable_read(
        &self,
        shell: &mut Shell,
    ) -> Result<(FileLock, Option<StateInner>), StateReadError> {
        let locked_file = self
            .filesystem
            .open_ro_shared_create(Self::STATE_FILENAME, shell, "Pavex's state file")
            .map_err(AcquireLockError)?;
        self._read(locked_file)
    }

    fn _read(
        &self,
        mut locked_file: FileLock,
    ) -> Result<(FileLock, Option<StateInner>), StateReadError> {
        let mut contents = String::new();
        locked_file
            .read_to_string(&mut contents)
            .map_err(|e| StateReadError::ReadError(e, Self::STATE_FILENAME))?;

        if contents.is_empty() {
            Ok((locked_file, None))
        } else {
            let contents = toml::from_str(&contents)
                .map_err(|e| StateReadError::CannotParse(e, Self::STATE_FILENAME))?;
            Ok((locked_file, Some(contents)))
        }
    }
}

#[derive(thiserror::Error, Debug)]
#[error("Failed to acquire a lock on Pavex's state file.")]
pub struct AcquireLockError(#[from] anyhow::Error);

#[derive(thiserror::Error, Debug)]
pub enum StateReadError {
    #[error("Failed to parse Pavex's state file, {1}: {0}")]
    CannotParse(#[source] toml::de::Error, &'static str),
    #[error("Failed to read Pavex's state file, {1}: {0}")]
    ReadError(#[source] std::io::Error, &'static str),
    #[error(transparent)]
    AcquireLock(#[from] AcquireLockError),
}
