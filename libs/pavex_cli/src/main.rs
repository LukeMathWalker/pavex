use anyhow::Context;
use cargo_like_utils::shell::Shell;
use std::fmt::{Display, Formatter};
use std::path::PathBuf;
use std::process::ExitCode;
use std::str::FromStr;

use clap::{Parser, Subcommand};
use pavex_cli::locator::PavexLocator;
use pavex_cli::package_graph::compute_package_graph;
use pavex_cli::pavexc::{get_or_install_from_graph, get_or_install_from_version};
use pavex_cli::state::State;
use pavexc_cli_client::commands::generate::BlueprintArgument;
use pavexc_cli_client::Client;
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
    /// Generate application runtime code according to an application blueprint.
    Generate {
        /// The source path for the serialized application blueprint.
        #[clap(short, long, value_parser)]
        blueprint: PathBuf,
        /// Optional. If provided, pavex will serialize diagnostic information about
        /// the application to the specified path.
        #[clap(long, value_parser)]
        diagnostics: Option<PathBuf>,
        /// The path to the directory that will contain the manifest and the source code for the generated application crate.  
        /// If the provided path is relative, it is interpreted as relative to the root of the current workspace.
        #[clap(short, long, value_parser)]
        output: PathBuf,
    },
    /// Scaffold a new Pavex project at <PATH>.
    New {
        /// The path of the new directory that will contain the project files.  
        ///
        /// If any of the intermediate directories in the path don't exist, they'll be created.
        #[arg(index = 1)]
        path: PathBuf,
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

fn main() -> Result<ExitCode, Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    miette::set_hook(Box::new(move |_| {
        let mut handler = pavex_miette::PavexMietteHandlerOpts::new();
        if cli.debug {
            handler = handler.with_cause_chain()
        } else {
            handler = handler.without_cause_chain()
        };
        // This is an undocumented feature that allows us to force set the width of the
        // terminal as seen by the graphical error handler.
        // This is useful for testing/doc-generation purposes.
        if let Ok(width) = std::env::var("PAVEX_TTY_WIDTH") {
            if let Ok(width) = width.parse::<usize>() {
                handler = handler.width(width);
            }
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

    let system_home_dir =
        xdg_home::home_dir().context("Failed to get the system home directory")?;
    let locator = PavexLocator::new(&system_home_dir);
    let mut shell = Shell::new();

    match cli.command {
        Commands::Generate {
            blueprint,
            diagnostics,
            output,
        } => generate(client, &locator, blueprint, diagnostics, output),
        Commands::New { path } => scaffold_project(client, &locator, &mut shell, path),
    }
}

#[tracing::instrument("Generate server sdk", skip(client, locator))]
fn generate(
    mut client: Client,
    locator: &PavexLocator,
    blueprint: PathBuf,
    diagnostics: Option<PathBuf>,
    output: PathBuf,
) -> Result<ExitCode, Box<dyn std::error::Error>> {
    // Match the version of the `pavexc` binary with the version of the `pavex` library
    // crate used in the current workspace.
    {
        let package_graph = compute_package_graph()
            .context("Failed to compute package graph for the current workspace")?;
        let pavexc_cli_path = get_or_install_from_graph(locator, &package_graph)
            .context("Failed to get or install the `pavexc` binary")?;
        client = client.pavexc_cli_path(pavexc_cli_path);
    }

    let blueprint = BlueprintArgument::Path(blueprint);
    let mut cmd = client.generate(blueprint, output);
    if let Some(diagnostics) = diagnostics {
        cmd = cmd.diagnostics_path(diagnostics)
    };
    cmd.execute()?;
    Ok(ExitCode::SUCCESS)
}

#[tracing::instrument("Scaffold new project", skip(client, locator))]
fn scaffold_project(
    mut client: Client,
    locator: &PavexLocator,
    shell: &mut Shell,
    path: PathBuf,
) -> Result<ExitCode, Box<dyn std::error::Error>> {
    {
        let version = State::new(locator)
            .get_current_toolchain(shell)
            .context("Failed to get the current toolchain")?;
        let pavexc_cli_path = get_or_install_from_version(locator, &version)
            .context("Failed to get or install the `pavexc` binary")?;
        client = client.pavexc_cli_path(pavexc_cli_path);
    }

    client.new_command(path).execute()?;
    Ok(ExitCode::SUCCESS)
}
