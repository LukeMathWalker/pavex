use std::{path::PathBuf, process::Command};

use crate::commands::errors::{InvocationError, NonZeroExitCode, SignalTermination};
use pavex::blueprint::Blueprint;

/// The configuration for `pavexc`'s `generate` command.
///
/// You can use [`Client::generate`] to start building the command configuration.
///
/// [`Client::generate`]: crate::Client::generate
pub struct GenerateBuilder {
    cmd: Command,
    diagnostics_path: Option<PathBuf>,
    blueprint: BlueprintArgument,
    output_directory: PathBuf,
    check: bool,
}

pub enum BlueprintArgument {
    Path(PathBuf),
    InMemory(Blueprint),
}

/// The representation of this command used in error messages.
static GENERATE_DEBUG_COMMAND: &str = "pavexc [...] generate [...]";

impl GenerateBuilder {
    pub(crate) fn new(
        cmd: Command,
        blueprint: BlueprintArgument,
        output_directory: PathBuf,
    ) -> Self {
        Self {
            diagnostics_path: None,
            blueprint,
            cmd,
            output_directory,
            check: false,
        }
    }

    /// Generate the runtime library for the application.
    ///
    /// This will invoke `pavexc` with the chosen configuration.
    /// It won't return until `pavexc` has finished running.
    ///
    /// If `pavexc` exits with a non-zero status code, this will return an error.
    pub fn execute(self) -> Result<(), GenerateError> {
        let mut cmd = self
            .command()
            .map_err(GenerateError::BlueprintPersistenceError)?;
        let status = cmd
            .status()
            .map_err(|e| InvocationError {
                source: e,
                command: GENERATE_DEBUG_COMMAND,
            })
            .map_err(GenerateError::InvocationError)?;
        if !status.success() {
            if let Some(code) = status.code() {
                return Err(GenerateError::NonZeroExitCode(NonZeroExitCode {
                    code,
                    command: GENERATE_DEBUG_COMMAND,
                }));
            } else {
                return Err(GenerateError::SignalTermination(SignalTermination {
                    command: GENERATE_DEBUG_COMMAND,
                }));
            }
        }
        Ok(())
    }

    /// Assemble the `std::process::Command` that will be used to invoke `pavexc`,
    /// but do not run it.
    /// It **will** persist the blueprint to a file, though.
    ///
    /// This method can be useful if you need to customize the command before running it.  
    /// If that's not your usecase, consider using [`GenerateBuilder::execute`] instead.
    pub fn command(mut self) -> Result<std::process::Command, BlueprintPersistenceError> {
        let bp_path = match self.blueprint {
            BlueprintArgument::Path(p) => p,
            BlueprintArgument::InMemory(ref bp) => {
                // TODO: Pass the blueprint via `stdin` instead of writing it to a file.
                let bp_path = self.output_directory.join("blueprint.ron");
                bp.persist(&bp_path)
                    .map_err(|source| BlueprintPersistenceError { source })?;
                bp_path
            }
        };

        self.cmd
            .arg("generate")
            .arg("-b")
            .arg(bp_path)
            .arg("-o")
            .arg(self.output_directory)
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit());

        if let Some(path) = self.diagnostics_path {
            self.cmd.arg("--diagnostics").arg(path);
        }
        if self.check {
            self.cmd.arg("--check");
        }
        Ok(self.cmd)
    }

    /// Set the path to the file that Pavex will use to serialize diagnostic
    /// information about the application.
    ///
    /// Diagnostics are primarily used for debugging the generator itself.
    ///
    /// If this is not set, Pavex will not persist any diagnostic information.
    pub fn diagnostics_path(mut self, path: PathBuf) -> Self {
        self.diagnostics_path = Some(path);
        self
    }

    /// Enable check mode.
    ///
    /// In check mode, `pavexc generate` verifies that the generated server SDK is up-to-date.  
    /// If it isn't, it returns an error without updating the SDK.
    pub fn check(mut self) -> Self {
        self.check = true;
        self
    }

    /// Disable check mode.
    ///
    /// `pavexc` will regenerate the server SDK and update it on disk if it is outdated.
    pub fn no_check(mut self) -> Self {
        self.check = false;
        self
    }
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum GenerateError {
    #[error(transparent)]
    InvocationError(InvocationError),
    #[error(transparent)]
    SignalTermination(SignalTermination),
    #[error(transparent)]
    NonZeroExitCode(NonZeroExitCode),
    #[error(transparent)]
    BlueprintPersistenceError(BlueprintPersistenceError),
}

#[derive(Debug, thiserror::Error)]
#[error("Failed to persist the blueprint to a file")]
pub struct BlueprintPersistenceError {
    #[source]
    source: anyhow::Error,
}
