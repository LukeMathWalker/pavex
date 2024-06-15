use crate::commands::errors::{InvocationError, NonZeroExitCode, SignalTermination};
use std::{path::PathBuf, process::Command, str::FromStr};

/// The name of a template to use when creating a new Pavex project.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TemplateName {
    /// A minimal API template.
    Api,
    /// The project template used by the Pavex quickstart guide.
    Quickstart,
}

impl TemplateName {
    pub fn as_str(&self) -> &str {
        match self {
            TemplateName::Api => "api",
            TemplateName::Quickstart => "quickstart",
        }
    }
}

impl FromStr for TemplateName {
    type Err = InvalidTemplateName;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "api" => Ok(TemplateName::Api),
            "quickstart" => Ok(TemplateName::Quickstart),
            s => Err(InvalidTemplateName {
                name: s.to_string(),
            }),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("`{name}` is not a valid template name. Use either `api` or `quickstart`.")]
pub struct InvalidTemplateName {
    pub(crate) name: String,
}

/// The configuration for `pavexc`'s `new` command.
///
/// You can use [`Client::new`] to start building the command configuration.
///
/// [`Client::new`]: crate::Client::new
pub struct NewBuilder {
    cmd: Command,
    template: TemplateName,
    path: PathBuf,
}

/// The representation of this command used in error messages.
static NEW_DEBUG_COMMAND: &str = "pavexc [...] new [...]";

impl NewBuilder {
    pub(crate) fn new(cmd: Command, path: PathBuf) -> Self {
        Self {
            cmd,
            path,
            template: TemplateName::Api,
        }
    }

    /// Set the template to use when creating a new Pavex project.
    pub fn template(mut self, template: TemplateName) -> Self {
        self.template = template;
        self
    }

    /// Scaffold a new Pavex project.
    ///
    /// This will invoke `pavexc` with the chosen configuration.
    /// It won't return until `pavexc` has finished running.
    ///
    /// If `pavexc` exits with a non-zero status code, this will return an error.
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

    /// Assemble the `std::process::Command` that will be used to invoke `pavexc`,
    /// but do not run it.
    ///
    /// This method can be useful if you need to customize the command before running it.
    /// If that's not your usecase, consider using [`GenerateBuilder::execute`] instead.
    pub fn command(mut self) -> Command {
        self.cmd
            .arg("new")
            .arg(self.path)
            .arg("--template")
            .arg(self.template.as_str())
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
