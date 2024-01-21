use anyhow::Context;
use cargo_like_utils::shell::Shell;
use std::fmt::{Display, Formatter};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::str::FromStr;

use clap::{Parser, Subcommand};
use pavex_cli::activation::check_activation;
use pavex_cli::cargo_install::{cargo_install, GitSourceRevision, Source};
use pavex_cli::cli_kind::CliKind;
use pavex_cli::confirmation::confirm;
use pavex_cli::locator::PavexLocator;
use pavex_cli::package_graph::compute_package_graph;
use pavex_cli::pavexc::{get_or_install_from_graph, get_or_install_from_version};
use pavex_cli::prebuilt::download_prebuilt;
use pavex_cli::state::State;
use pavex_cli::utils;
use pavex_cli::version::latest_released_version;
use pavexc_cli_client::commands::generate::{BlueprintArgument, GenerateError};
use pavexc_cli_client::commands::new::NewError;
use pavexc_cli_client::Client;
use semver::Version;
use tracing_chrome::{ChromeLayerBuilder, FlushGuard};
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[clap(author, version = VERSION, about, long_about = None)]
struct Cli {
    /// Pavex will expose the full error chain when reporting diagnostics.
    ///
    /// It will also emit tracing output, both to stdout and to disk.
    /// The file serialized on disk (`trace-[...].json`) can be opened in
    /// Google Chrome by visiting chrome://tracing for further analysis.
    #[clap(long, env = "PAVEX_DEBUG")]
    debug: bool,
    #[clap(long, env = "PAVEX_COLOR", default_value_t = Color::Auto)]
    color: Color,
    #[clap(subcommand)]
    command: Commands,
}

// Same structure used by `cargo --version`.
static VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), " (", env!("VERGEN_GIT_SHA"), ")");

#[derive(Copy, Clone, Debug)]
enum Color {
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

#[derive(Subcommand)]
enum Commands {
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

#[derive(Subcommand)]
enum SelfCommands {
    /// Download and install a newer version of Pavex CLI, if available.
    Update,
    /// Uninstall Pavex CLI and remove all its dependencies and artifacts.
    Uninstall {
        /// Don't ask for confirmation before uninstalling Pavex CLI.
        #[clap(short, long, value_parser)]
        y: bool,
    },
}

fn init_telemetry() -> FlushGuard {
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_file(false)
        .with_target(false)
        .with_span_events(FmtSpan::NEW | FmtSpan::EXIT)
        .with_timer(tracing_subscriber::fmt::time::uptime());
    let (chrome_layer, guard) = ChromeLayerBuilder::new().include_args(true).build();
    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info,pavexc=trace"))
        .unwrap();

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .with(chrome_layer)
        .init();
    guard
}

fn main() -> Result<ExitCode, miette::Error> {
    let cli = Cli::parse();
    miette::set_hook(Box::new(move |_| {
        let mut handler = pavex_miette::PavexMietteHandlerOpts::new();
        if cli.debug {
            handler = handler.with_cause_chain()
        } else {
            handler = handler.without_cause_chain()
        };
        if let Some(width) = pavex_cli::env::tty_width() {
            handler = handler.width(width);
        }
        match cli.color {
            Color::Auto => {}
            Color::Always => {
                handler = handler.color(true);
            }
            Color::Never => {
                handler = handler.color(false);
            }
        }
        Box::new(handler.build())
    }))
    .unwrap();
    let _guard = if cli.debug {
        Some(init_telemetry())
    } else {
        None
    };

    let color_profile = match cli.color {
        Color::Auto => pavexc_cli_client::config::Color::Auto,
        Color::Always => pavexc_cli_client::config::Color::Always,
        Color::Never => pavexc_cli_client::config::Color::Never,
    };
    let mut client = Client::new().color(color_profile);
    if cli.debug {
        client = client.debug();
    } else {
        client = client.no_debug();
    }

    let system_home_dir = xdg_home::home_dir().ok_or_else(|| {
        miette::miette!("Failed to get the system home directory from the environment")
    })?;
    let locator = PavexLocator::new(&system_home_dir);
    let mut shell = Shell::new();

    match cli.command {
        Commands::Generate {
            blueprint,
            diagnostics,
            output,
        } => {
            check_activation(&State::new(&locator), &mut shell).map_err(utils::anyhow2miette)?;
            generate(&mut shell, client, &locator, blueprint, diagnostics, output)
        }
        Commands::New { path } => {
            check_activation(&State::new(&locator), &mut shell).map_err(utils::anyhow2miette)?;
            scaffold_project(client, &locator, &mut shell, path)
        }
        Commands::Self_ { command } => {
            // You should always be able to run `self` commands, even if Pavex has
            // not been activated yet.
            match command {
                SelfCommands::Update => update(&mut shell),
                SelfCommands::Uninstall { y } => uninstall(&mut shell, !y, locator),
            }
        }
    }
    .map_err(utils::anyhow2miette)
}

#[tracing::instrument("Generate server sdk", skip(client, locator, shell))]
fn generate(
    shell: &mut Shell,
    mut client: Client,
    locator: &PavexLocator,
    blueprint: PathBuf,
    diagnostics: Option<PathBuf>,
    output: PathBuf,
) -> Result<ExitCode, anyhow::Error> {
    let pavexc_cli_path = if let Some(pavexc_override) = pavex_cli::env::pavexc_override() {
        pavexc_override
    } else {
        // Match the version of the `pavexc` binary with the version of the `pavex` library
        // crate used in the current workspace.
        let package_graph = compute_package_graph()
            .context("Failed to compute package graph for the current workspace")?;
        get_or_install_from_graph(shell, locator, &package_graph)
            .context("Failed to get or install the `pavexc` binary")?
    };
    client = client.pavexc_cli_path(pavexc_cli_path);

    let blueprint = BlueprintArgument::Path(blueprint);
    let mut cmd = client.generate(blueprint, output);
    if let Some(diagnostics) = diagnostics {
        cmd = cmd.diagnostics_path(diagnostics)
    };

    match cmd.execute() {
        Ok(()) => Ok(ExitCode::SUCCESS),
        Err(GenerateError::NonZeroExitCode(e)) => Ok(ExitCode::from(e.code as u8)),
        Err(e) => Err(e.into()),
    }
}

#[tracing::instrument("Scaffold new project", skip(client, locator, shell))]
fn scaffold_project(
    mut client: Client,
    locator: &PavexLocator,
    shell: &mut Shell,
    path: PathBuf,
) -> Result<ExitCode, anyhow::Error> {
    let pavexc_cli_path = if let Some(pavexc_override) = pavex_cli::env::pavexc_override() {
        pavexc_override
    } else {
        let version = State::new(locator)
            .get_current_toolchain(shell)
            .context("Failed to get the current toolchain")?;
        get_or_install_from_version(shell, locator, &version)
            .context("Failed to get or install the `pavexc` binary")?
    };

    client = client.pavexc_cli_path(pavexc_cli_path);

    match client.new_command(path).execute() {
        Ok(()) => Ok(ExitCode::SUCCESS),
        Err(NewError::NonZeroExitCode(e)) => Ok(ExitCode::from(e.code as u8)),
        Err(e) => Err(e.into()),
    }
}

#[tracing::instrument("Uninstall Pavex CLI", skip(shell, locator))]
fn uninstall(
    shell: &mut Shell,
    must_prompt_user: bool,
    locator: PavexLocator,
) -> Result<ExitCode, anyhow::Error> {
    shell.status("Thanks", "for hacking with Pavex!")?;
    if must_prompt_user {
        shell.warn(
            "This process will uninstall Pavex and all its associated data from your system.",
        )?;
        let continue_ = confirm("\nDo you wish to continue? (y/N)", false)?;
        if !continue_ {
            shell.status("Abort", "Uninstalling Pavex CLI")?;
            return Ok(ExitCode::SUCCESS);
        }
    }

    shell.status("Uninstalling", "Pavex")?;
    if let Err(e) = fs_err::remove_dir_all(locator.root_dir()) {
        if ErrorKind::NotFound != e.kind() {
            Err(e).context("Failed to remove Pavex data")?;
        }
    }
    self_replace::self_delete().context("Failed to delete the current Pavex CLI binary")?;
    shell.status("Uninstalled", "Pavex")?;

    Ok(ExitCode::SUCCESS)
}

#[tracing::instrument("Update Pavex CLI", skip(shell))]
fn update(shell: &mut Shell) -> Result<ExitCode, anyhow::Error> {
    shell.status("Checking", "for updates to Pavex CLI")?;
    let latest_version = latest_released_version()?;
    let current_version = pavex_cli::env::version();
    if latest_version <= current_version {
        shell.status(
            "Up to date",
            format!("{current_version} is the most recent version"),
        )?;
        return Ok(ExitCode::SUCCESS);
    }

    shell.status(
        "Update available",
        format!("You're running {current_version}, but {latest_version} is available"),
    )?;

    let new_cli_path = tempfile::NamedTempFile::new()
        .context("Failed to create a temporary file to download the new Pavex CLI binary")?;
    download_or_compile(shell, CliKind::Pavex, &latest_version, new_cli_path.path())?;
    self_replace::self_replace(new_cli_path.path())
        .context("Failed to replace the current Pavex CLI with the newly downloaded version")?;

    Ok(ExitCode::SUCCESS)
}

fn download_or_compile(
    shell: &mut Shell,
    kind: CliKind,
    version: &Version,
    destination: &Path,
) -> Result<(), anyhow::Error> {
    let _ = shell.status(
        "Downloading",
        format!("prebuilt `{}@{version}` binary", kind.binary_target_name()),
    );
    match download_prebuilt(destination, kind, version) {
        Ok(_) => {
            let _ = shell.status(
                "Downloaded",
                format!("prebuilt `{}@{version}` binary", kind.binary_target_name()),
            );
            return Ok(());
        }
        Err(e) => {
            let _ = shell.warn("Download failed: {e}.\nI'll try compiling from source instead.");
            tracing::warn!(
                error.msg = %e,
                error.cause = ?e,
                "Failed to download prebuilt `{}` binary. I'll try to build it from source instead.", kind.binary_target_name()
            );
        }
    }

    let _ = shell.status(
        "Compiling",
        format!("`{}@{version}` from source", kind.package_name()),
    );
    cargo_install(
        Source::Git {
            url: "https://github.com/LukeMathWalker/pavex".into(),
            rev: GitSourceRevision::Tag(version.to_string()),
        },
        kind,
        destination,
    )?;
    let _ = shell.status(
        "Compiled",
        format!("`{}@{version}` from source", kind.package_name()),
    );
    Ok(())
}
