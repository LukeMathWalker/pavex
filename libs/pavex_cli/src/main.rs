use std::fmt::{Display, Formatter};
use std::path::PathBuf;
use std::process::ExitCode;
use std::str::FromStr;

use clap::{Parser, Subcommand};
use miette::{GraphicalTheme, ThemeCharacters, ThemeStyles};
use owo_colors::OwoColorize;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

use pavex::App;
use pavex_builder::Blueprint;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    /// Expose inner details in case of an error.
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
        /// The target directory for the generated application crate.  
        /// The path is interpreted as relative to the root of the current workspace.
        #[clap(short, long, value_parser)]
        output: PathBuf,
    },
}

fn init_telemetry() {
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_file(false)
        .with_target(false)
        .with_span_events(FmtSpan::NEW | FmtSpan::EXIT)
        .with_timer(tracing_subscriber::fmt::time::uptime());
    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap();

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .init();
}

fn main() -> Result<ExitCode, Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    miette::set_hook(Box::new(move |_| {
        // TODO: Bring in all miette's auto-detection logic.
        let mut handler = pavex_miette::GraphicalReportHandler::new_themed(GraphicalTheme {
            characters: ThemeCharacters::unicode(),
            styles: ThemeStyles::ansi(),
        });
        handler = handler.with_width(80);
        if cli.debug {
            handler = handler.with_cause_chain()
        } else {
            handler = handler.without_cause_chain()
        };
        Box::new(handler)
    }))
    .unwrap();
    if cli.debug {
        init_telemetry();
    }
    match cli.command {
        Commands::Generate {
            blueprint,
            diagnostics,
            output,
        } => {
            let blueprint = Blueprint::load(&blueprint)?;
            let app = match App::build(blueprint) {
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
            assert!(
                output.is_relative(),
                "The output path must be relative to the root of the current `cargo` workspace."
            );
            let generated_app = app.codegen()?;
            generated_app.persist(&output)?;
        }
    }
    Ok(ExitCode::SUCCESS)
}
