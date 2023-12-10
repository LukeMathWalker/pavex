use std::fmt::{Display, Formatter};
use std::path::PathBuf;
use std::process::ExitCode;
use std::str::FromStr;

use anyhow::Context;
use cargo_generate::{GenerateArgs, TemplatePath};
use clap::{Parser, Subcommand};
use owo_colors::OwoColorize;
use supports_color::Stream;
use tracing_chrome::{ChromeLayerBuilder, FlushGuard};
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

use pavex::blueprint::Blueprint;
use pavexc::App;

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
        } => generate(blueprint, diagnostics, output, cli.color),
        Commands::New { path } => scaffold_project(path),
    }
}

#[tracing::instrument("Generate server sdk")]
fn generate(
    blueprint: PathBuf,
    diagnostics: Option<PathBuf>,
    output: PathBuf,
    color_profile: Color,
) -> Result<ExitCode, Box<dyn std::error::Error>> {
    let color_on_stderr = use_color_on_stderr(color_profile);
    let blueprint = Blueprint::load(&blueprint)?;
    // We use the path to the generated application crate as a fingerprint for the project.
    let project_fingerprint = output.to_string_lossy().into_owned();
    let app = match App::build(blueprint, project_fingerprint) {
        Ok(a) => a,
        Err(errors) => {
            for e in errors {
                if color_on_stderr {
                    eprintln!("{}: {e:?}", "ERROR".bold().red());
                } else {
                    eprintln!("ERROR: {e:?}");
                };
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

fn use_color_on_stderr(color_profile: Color) -> bool {
    match color_profile {
        Color::Auto => supports_color::on(Stream::Stderr).is_some(),
        Color::Always => true,
        Color::Never => false,
    }
}

static TEMPLATE_DIR: include_dir::Dir =
    include_dir::include_dir!("$CARGO_MANIFEST_DIR/../../template");

fn scaffold_project(path: PathBuf) -> Result<ExitCode, Box<dyn std::error::Error>> {
    let name = path
        .file_name()
        .ok_or_else(|| {
            anyhow::anyhow!("Failed to derive a project name from the provided path")
        })?
        .to_str()
        .ok_or_else(|| {
            anyhow::anyhow!("The last segment of the provided path must be valid UTF8 to generate a valid project name")
        })?
        .to_string();

    let target_directory =
        std::env::temp_dir().join(format!("pavex-template-{}", env!("VERGEN_GIT_SHA")));
    TEMPLATE_DIR
        .extract(&target_directory)
        .context("Failed to save Pavex's template to a temporary directory")?;

    let generate_args = GenerateArgs {
        template_path: TemplatePath {
            path: Some(
                target_directory
                    .to_str()
                    .context("Failed to convert the template path to a UTF8 string")?
                    .into(),
            ),
            ..Default::default()
        },
        destination: path
            .parent()
            .map(|p| {
                use path_absolutize::Absolutize;

                p.absolutize().map(|p| p.to_path_buf())
            })
            .transpose()
            .context("Failed to convert destination path to an absolute path")?,
        name: Some(name),
        force_git_init: true,
        ..Default::default()
    };
    cargo_generate::generate(generate_args)
        .context("Failed to scaffold the project from Pavex's default template")?;
    return Ok(ExitCode::SUCCESS);
}
