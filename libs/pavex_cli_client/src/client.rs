use std::{path::PathBuf, process::Command};

use pavex::blueprint::Blueprint;

use crate::commands::generate::GenerateBuilder;

/// A fluent API for configuring and executing `pavex_cli`'s commands.
#[derive(Clone, Debug)]
pub struct Client {
    pavex_cli_path: Option<PathBuf>,
    color: Color,
    debug: bool,
}

impl Client {
    /// Create a new `Client` with the default configuration.
    pub fn new() -> Self {
        Self {
            pavex_cli_path: None,
            color: Color::Auto,
            debug: false,
        }
    }

    /// Convert this `Client` into a `std::process::Command` that will run `pavex_cli`
    /// with the chosen configuration.
    fn command(self) -> Command {
        let pavex_path = self.pavex_cli_path.unwrap_or_else(|| "pavex_cli".into());
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

        cmd
    }

    /// Start building the configuration for the code-generator.
    ///
    /// You must specify:
    ///
    /// - The `Blueprint` for the application that you want to generate;
    /// - The directory where the generated code should be written.
    pub fn generate(self, blueprint: Blueprint, output_directory: PathBuf) -> GenerateBuilder {
        let cmd = self.command();
        GenerateBuilder::new(cmd, blueprint, output_directory)
    }
}

/// Setters for optional configuration knobs on `Client`.
impl Client {
    /// Set the path to the `pavex_cli` executable.
    ///
    /// If this is not set, we will assume that `pavex_cli` is in the `PATH`.
    pub fn pavex_cli_path(mut self, path: PathBuf) -> Self {
        self.pavex_cli_path = Some(path);
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
    /// This will print additional debug information when running `pavex_cli` commands.
    pub fn debug(mut self) -> Self {
        self.debug = true;
        self
    }

    /// Disable debug mode.
    ///
    /// `pavex_cli` will not print additional debug information when running commands.  
    /// This is the default behaviour.
    pub fn no_debug(mut self) -> Self {
        self.debug = false;
        self
    }
}

/// Control whether to use colors in the output of Pavex's code generator.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum Color {
    /// Automatically determine whether to use colors.
    Auto,
    /// Always use colors.
    Always,
    /// Never use colors.
    Never,
}
