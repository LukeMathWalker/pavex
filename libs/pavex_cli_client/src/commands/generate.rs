use anyhow::Context;
use pavex::blueprint::Blueprint;
use std::{path::PathBuf, process::Command};

/// The configuration for `pavex_cli`'s `generate` command.
///
/// You can use [`Client::generate`] to start building the command configuration.
///
/// [`Client::generate`]: crate::Client::generate
pub struct GenerateBuilder {
    cmd: Command,
    diagnostics_path: Option<PathBuf>,
    blueprint: Blueprint,
    output_directory: PathBuf,
}

impl GenerateBuilder {
    pub(crate) fn new(cmd: Command, blueprint: Blueprint, output_directory: PathBuf) -> Self {
        Self {
            diagnostics_path: None,
            blueprint,
            cmd,
            output_directory,
        }
    }

    /// Generate the runtime library for the application.
    ///
    /// This will invoke `pavex_cli` with the chosen configuration.
    /// It won't return until `pavex_cli` has finished running.
    ///
    /// If `pavex_cli` exits with a non-zero status code, this will return an error.
    pub fn execute(self) -> Result<(), anyhow::Error> {
        let mut cmd = self.command()?;
        let status = cmd
            .status()
            .context("Failed to invoke `pavex_cli [...] generate [...]`")?;
        if !status.success() {
            if let Some(code) = status.code() {
                anyhow::bail!("`pavex_cli` exited with a non-zero status code: {}", code);
            } else {
                anyhow::bail!("`pavex_cli` was terminated by a signal");
            }
        }
        Ok(())
    }

    /// Assemble the `std::process::Command` that will be used to invoke `pavex_cli`,
    /// but do not run it.
    /// It **will** persist the blueprint to a file, though.
    ///
    /// This method can be useful if you need to customize the command before running it.  
    /// If that's not your usecase, consider using [`GenerateBuilder::execute`] instead.
    pub fn command(mut self) -> Result<std::process::Command, anyhow::Error> {
        // TODO: Pass the blueprint via `stdin` instead of writing it to a file.
        let bp_path = self.output_directory.join("blueprint.ron");
        self.blueprint.persist(&bp_path)?;

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
}
