use crate::commands::errors::{InvocationError, NonZeroExitCode, SignalTermination};
use std::{path::PathBuf, process::Command};

/// The configuration for `pavex`'s `new` command.
///
/// You can use [`Client::new`] to start building the command configuration.
///
/// [`Client::new`]: crate::Client::new
pub struct NewBuilder {
    cmd: Command,
    path: PathBuf,
}

/// The representation of this command used in error messages.
static NEW_DEBUG_COMMAND: &str = "pavex [...] new [...]";

impl NewBuilder {
    pub(crate) fn new(cmd: Command, path: PathBuf) -> Self {
        Self { cmd, path }
    }

    /// Scaffold a new Pavex project.
    ///
    /// This will invoke `pavex` with the chosen configuration.
    /// It won't return until `pavex` has finished running.
    ///
    /// If `pavex` exits with a non-zero status code, this will return an error.
    pub fn execute(self) -> Result<(), NewError> {
        let mut cmd = self.command();
        let status = cmd
            .status()
            .map_err(|e| InvocationError {
                source: e,
                command: NEW_DEBUG_COMMAND,
            })
            .map_err(NewError::InvocationError)?;
        if !status.success() {
            if let Some(code) = status.code() {
                return Err(NewError::NonZeroExitCode(NonZeroExitCode {
                    code,
                    command: NEW_DEBUG_COMMAND,
                }));
            } else {
                return Err(NewError::SignalTermination(SignalTermination {
                    command: NEW_DEBUG_COMMAND,
                }));
            }
        }
        Ok(())
    }

    /// Assemble the `std::process::Command` that will be used to invoke `pavex`,
    /// but do not run it.
    ///
    /// This method can be useful if you need to customize the command before running it.
    /// If that's not your usecase, consider using [`GenerateBuilder::execute`] instead.
    pub fn command(mut self) -> Command {
        self.cmd
            .arg("new")
            .arg(self.path)
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit());
        self.cmd
    }
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum NewError {
    #[error(transparent)]
    InvocationError(InvocationError),
    #[error(transparent)]
    SignalTermination(SignalTermination),
    #[error(transparent)]
    NonZeroExitCode(NonZeroExitCode),
}
