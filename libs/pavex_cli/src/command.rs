use clap::{Parser, Subcommand};
use clap_stdin::MaybeStdin;
use redact::Secret;
use std::fmt::{Display, Formatter};
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Parser)]
#[clap(author, version = VERSION, about, long_about = None)]
pub struct Cli {
    /// Pavex will expose the full error chain when reporting diagnostics.
    ///
    /// It will also emit tracing output, both to stdout and to disk.
    /// The file serialized on disk (`trace-[...].json`) can be opened in
    /// Google Chrome by visiting chrome://tracing for further analysis.
    #[clap(long, env = "PAVEX_DEBUG")]
    pub debug: bool,
    #[clap(long, env = "PAVEX_COLOR", default_value_t = Color::Auto)]
    pub color: Color,
    #[clap(subcommand)]
    pub command: Command,
}

// Same structure used by `cargo --version`.
static VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), " (", env!("VERGEN_GIT_SHA"), ")");

#[derive(Copy, Clone, Debug)]
pub enum Color {
    Auto,
    Always,
    Never,
}

impl Display for Color {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Color::Auto => write!(f, "auto"),
            Color::Always => write!(f, "always"),
            Color::Never => write!(f, "never"),
        }
    }
}

impl FromStr for Color {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "auto" => Ok(Color::Auto),
            "always" => Ok(Color::Always),
            "never" => Ok(Color::Never),
            s => Err(anyhow::anyhow!("Invalid color setting: {}", s)),
        }
    }
}

impl From<Color> for pavexc_cli_client::config::Color {
    fn from(value: Color) -> Self {
        match value {
            Color::Auto => pavexc_cli_client::config::Color::Auto,
            Color::Always => pavexc_cli_client::config::Color::Always,
            Color::Never => pavexc_cli_client::config::Color::Never,
        }
    }
}

#[derive(Subcommand)]
pub enum Command {
    /// Generate the server SDK code for an application blueprint.
    Generate {
        /// The source path for the serialized application blueprint.
        #[clap(short, long, value_parser)]
        blueprint: PathBuf,
        /// Optional.
        /// If provided, Pavex will serialize diagnostic information about
        /// the application to the specified path.
        #[clap(long, value_parser)]
        diagnostics: Option<PathBuf>,
        #[clap(long)]
        /// Verify that the generated server SDK is up-to-date.
        /// If it isn't, `pavex` will return an error without updating
        /// the server SDK code.
        check: bool,
        /// The directory that will contain the newly generated server SDK crate.
        /// If the directory path is relative,
        /// it is interpreted as relative to the root of the current workspace.
        #[clap(short, long, value_parser)]
        output: PathBuf,
    },
    /// Scaffold a new Pavex project at <PATH>.
    New {
        /// The directory that will contain the project files.
        ///
        /// If any of the intermediate directories in the path don't exist, they'll be created.
        #[arg(index = 1)]
        path: PathBuf,
    },
    /// Modify the installation of the Pavex CLI.
    #[command(name = "self")]
    Self_ {
        #[clap(subcommand)]
        command: SelfCommands,
    },
}

impl Command {
    /// Returns `true` if the command requires a valid activation key.
    pub(crate) fn needs_activation_key(&self) -> bool {
        match self {
            Command::Generate { check, .. } => !check,
            Command::New { .. } => true,
            Command::Self_ { .. } => false,
        }
    }
}

#[derive(Subcommand)]
pub enum SelfCommands {
    /// Download and install a newer version of Pavex CLI, if available.
    Update,
    /// Prepare the system to use Pavex CLI.
    ///
    /// Pavex CLI requires other software to be installed on your
    /// machine to work as expected: `rustup`, `cargo-px`, Rust
    /// nightly toolchain, the `rustdoc-json` toolchain component.
    ///
    /// This command checks that this software is installed and
    /// located where Pavex CLI expects it to be.
    /// If it isn't, it offers to install it for you.
    Setup,
    /// Uninstall Pavex CLI and remove all its dependencies and artifacts.
    Uninstall {
        /// Don't ask for confirmation before uninstalling Pavex CLI.
        #[clap(short, long, value_parser)]
        y: bool,
    },
    Activate {
        /// The activation key for Pavex.
        /// You can find the activation key for the beta program in Pavex's Discord server,
        /// in the #announcements channel.
        #[arg(index = 1, env = "PAVEX_ACTIVATION_KEY")]
        key: Option<MaybeStdin<Secret<String>>>,
    },
}
