use clap::{Parser, Subcommand};
use clap_stdin::MaybeStdin;
use pavexc_cli_client::commands::new::TemplateName;
use redact::Secret;
use std::fmt::{Display, Formatter};
use std::path::PathBuf;
use std::str::FromStr;

const INTROSPECTION_HEADING: &str = "Introspection";

#[derive(Parser)]
#[clap(
    author,
    version = VERSION, about,
    long_about = None,
    after_long_help = "Use `pavex -h` rather than `pavex --help` for a more concise summary of the available options."
)]
pub struct Cli {
    #[clap(long, env = "PAVEX_COLOR", default_value_t = Color::Auto)]
    /// Color settings for the CLI output: auto, always, never.
    pub color: Color,
    #[clap(subcommand)]
    pub command: Command,
    #[clap(
        long,
        env = "PAVEX_DEBUG",
        help = "Pavex will expose the full error chain when reporting diagnostics.",
        long_help = "Pavex will expose the full error chain when reporting diagnostics.\nSet `PAVEX_DEBUG=1` to enable this option."
    )]
    pub debug: bool,
    #[clap(
        long,
        env = "PAVEX_LOG",
        help_heading = Some(INTROSPECTION_HEADING),
        hide_short_help = true,
        hide_env = true,
        long_help = "Pavex will emit internal logs to the console.\nSet `PAVEX_LOG=true` to enable this option using an environment variable."
    )]
    pub log: bool,
    #[clap(
        long,
        env = "PAVEX_LOG_FILTER",
        help_heading = Some(INTROSPECTION_HEADING),
        hide_short_help = true,
        hide_env = true,
        long_help = "Control which logs are emitted if `--log` or `--perf-profile` are enabled.\nIf no filter is specified, Pavex will default to `info,pavex=trace`."
    )]
    pub log_filter: Option<String>,
    #[clap(
        long,
        env = "PAVEX_PERF_PROFILE",
        help_heading = Some(INTROSPECTION_HEADING),
        hide_short_help = true,
        hide_env = true,
        long_help = "Pavex will serialize to disk tracing information to profile command execution.\nThe file (`trace-[...].json`) can be opened using https://ui.perfetto.dev/ or in Google Chrome by visiting chrome://tracing.\nSet `PAVEX_PERF_PROFILE=true` to enable this option using an environment variable."
    )]
    pub perf_profile: bool,
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
        /// The template that should be used to scaffold the project.
        /// It must be one of the following: `api`, `quickstart`.
        ///
        /// If not provided, Pavex will use the `api` template.
        #[clap(short, long, value_parser, default_value = "api")]
        template: TemplateName,
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
    /// Activate your Pavex installation.
    Activate {
        /// The activation key for Pavex.
        /// You can find the activation key for the beta program in Pavex's Discord server,
        /// in the #announcements channel.
        #[arg(index = 1, env = "PAVEX_ACTIVATION_KEY")]
        key: Option<MaybeStdin<Secret<String>>>,
    },
}
