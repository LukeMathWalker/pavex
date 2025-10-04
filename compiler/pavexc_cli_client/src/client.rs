use std::{path::PathBuf, process::Command};

use crate::commands::generate::{BlueprintArgument, GenerateBuilder};
use crate::commands::new::NewBuilder;
use crate::config::Color;

/// A fluent API for configuring and executing `pavexc`'s CLI commands.
#[derive(Clone, Debug)]
pub struct Client {
    pavexc_cli_path: Option<PathBuf>,
    color: Color,
    debug: bool,
    log: bool,
    log_filter: Option<String>,
    perf_profile: bool,
}

impl Default for Client {
    fn default() -> Self {
        Self {
            pavexc_cli_path: None,
            color: Color::Auto,
            debug: false,
            log: false,
            log_filter: None,
            perf_profile: false,
        }
    }
}

impl Client {
    /// Create a new `Client` with the default configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Convert this `Client` into a `std::process::Command` that will run `pavexc`
    /// with the chosen configuration.
    fn command(self) -> Command {
        let pavex_path = self.pavexc_cli_path.unwrap_or_else(|| "pavexc".into());
        let mut cmd = Command::new(pavex_path);

        match self.color {
            Color::Auto => {}
            Color::Always => {
                cmd.arg("--color").arg("always");
            }
            Color::Never => {
                cmd.arg("--color").arg("never");
            }
        }

        if self.debug {
            cmd.arg("--debug");
        }

        if self.log {
            cmd.arg("--log");
        }

        if let Some(filter) = self.log_filter {
            cmd.arg("--log-filter").arg(filter);
        }

        if self.perf_profile {
            cmd.arg("--perf-profile");
        }

        cmd
    }

    /// Start building the configuration for the code-generator.
    ///
    /// You must specify:
    ///
    /// - The `Blueprint` for the application that you want to generate;
    /// - The directory where the generated code should be written.
    pub fn generate(
        self,
        blueprint: BlueprintArgument,
        output_directory: PathBuf,
    ) -> GenerateBuilder {
        let cmd = self.command();
        GenerateBuilder::new(cmd, blueprint, output_directory)
    }

    /// Start building the configuration for the `new` command.
    ///
    /// You must specify the path where the new project should be created.
    pub fn new_command(self, path: PathBuf) -> NewBuilder {
        let cmd = self.command();
        NewBuilder::new(cmd, path)
    }
}

/// Setters for optional configuration knobs on `Client`.
impl Client {
    /// Set the path to the `pavexc` executable.
    ///
    /// If this is not set, we will assume that `pavexc` is in the `PATH`.
    pub fn pavexc_cli_path(mut self, path: PathBuf) -> Self {
        self.pavexc_cli_path = Some(path);
        self
    }

    /// Set whether to use colors in the output of Pavex's code generator.
    ///
    /// If this is not set, Pavex will automatically determine whether to use colors or not.
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Enable debug mode.
    ///
    /// `pavexc` will expose the full error chain when reporting diagnostics.
    pub fn debug(mut self) -> Self {
        self.debug = true;
        self
    }

    /// Disable debug mode.
    ///
    /// `pavexc` will not expose the full error chain when reporting diagnostics.
    /// This is the default behaviour.
    pub fn no_debug(mut self) -> Self {
        self.debug = false;
        self
    }

    /// Enable logging.
    ///
    /// `pavexc` will emit internal log messages to the console.
    pub fn log(mut self) -> Self {
        self.log = true;
        self
    }

    /// Disable logging.
    ///
    /// `pavexc` will not emit internal log messages to the console.
    /// This is the default behaviour.
    pub fn no_log(mut self) -> Self {
        self.log = false;
        self
    }

    /// Set the log filter.
    ///
    /// Control which logs are emitted if `--log` or `--perf-profile` are enabled.
    /// If no filter is specified, Pavex will default to `info,pavexc=trace`.
    pub fn log_filter(mut self, filter: String) -> Self {
        self.log_filter = Some(filter);
        self
    }

    /// Enable performance profiling.
    ///
    /// `pavexc` will serialize to disk tracing information to profile command execution.
    pub fn perf_profile(mut self) -> Self {
        self.perf_profile = true;
        self
    }

    /// Disable performance profiling.
    ///
    /// `pavexc` will not serialize to disk tracing information to profile command execution.
    /// This is the default behaviour.
    pub fn no_perf_profile(mut self) -> Self {
        self.perf_profile = false;
        self
    }
}
