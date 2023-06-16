use std::fmt::{Display, Formatter};
use std::path::PathBuf;
use std::process::ExitCode;
use std::str::FromStr;

use clap::{Parser, Subcommand};
use owo_colors::OwoColorize;
use tracing_chrome::{ChromeLayerBuilder, FlushGuard};
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

use pavex::blueprint::Blueprint;
use pavexc::App;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
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

#[derive(Clone, Debug)]
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
}

fn init_telemetry() -> FlushGuard {
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_file(false)
        .with_target(false)
        .with_span_events(FmtSpan::NEW | FmtSpan::EXIT)
        .with_timer(tracing_subscriber::fmt::time::uptime());
    let (chrome_layer, guard) = ChromeLayerBuilder::new().include_args(true).build();
    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info,pavex=trace"))
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
    match cli.command {
        Commands::Generate {
            blueprint,
            diagnostics,
            output,
        } => generate(blueprint, diagnostics, output),
    }
}

#[tracing::instrument("Generate API server sdk")]
fn generate(
    blueprint: PathBuf,
    diagnostics: Option<PathBuf>,
    output: PathBuf,
) -> Result<ExitCode, Box<dyn std::error::Error>> {
    let blueprint = Blueprint::load(&blueprint)?;
    // We use the path to the generated application crate as a fingerprint for the project.
    let project_fingerprint = output.to_string_lossy().into_owned();
    let app = match App::build(blueprint, project_fingerprint) {
        Ok(a) => a,
        Err(errors) => {
            for e in errors {
                eprintln!("{}: {:?}", "ERROR".bold().red(), e);
            }
            return Ok(ExitCode::FAILURE);
        }
    };
    if let Some(diagnostic_path) = diagnostics {
        app.diagnostic_representation()
            .persist_flat(&diagnostic_path)?;
    }
    let generated_app = app.codegen()?;
    generated_app.persist(&output)?;
    Ok(ExitCode::SUCCESS)
}
