use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::str::FromStr;

mod formatter;
mod telemetry;

use anyhow::Context;
use cargo_like_utils::shell::Shell;
use clap::{Parser, Subcommand};
use formatter::ReversedFull;
use generate_from_path::GenerateArgs;
use liquid_core::Value;
use miette::Severity;
use owo_colors::OwoColorize;
use pavex_cli_deps::{verify_installation, IfAutoinstallable, RustdocJson, RustupToolchain};
use pavexc::{App, AppWriter, DEFAULT_DOCS_TOOLCHAIN};
use pavexc_cli_client::commands::new::TemplateName;
use supports_color::Stream;
use telemetry::Filtered;
use tracing_chrome::{ChromeLayerBuilder, FlushGuard};
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

const INTROSPECTION_HEADING: &str = "Introspection";

#[derive(Parser, Debug)]
#[clap(author, version = VERSION, about, long_about = None)]
struct Cli {
    #[clap(long, env = "PAVEXC_COLOR", default_value_t = Color::Auto)]
    color: Color,
    #[clap(subcommand)]
    command: Commands,
    #[clap(
        long,
        env = "PAVEXC_DEBUG",
        help = "Pavexc will expose the full error chain when reporting diagnostics.",
        long_help = "Pavexc will expose the full error chain when reporting diagnostics.\nSet `PAVEXC_DEBUG=1` to enable this option."
    )]
    pub debug: bool,
    #[clap(
        long,
        env = "PAVEXC_LOG",
        help_heading = Some(INTROSPECTION_HEADING),
        hide_short_help = true,
        hide_env = true,
        long_help = "Pavexc will emit internal logs to the console.\nSet `PAVEXC_LOG=true` to enable this option using an environment variable."
    )]
    pub log: bool,
    #[clap(
        long,
        env = "PAVEXC_LOG_FILTER",
        help_heading = Some(INTROSPECTION_HEADING),
        hide_short_help = true,
        hide_env = true,
        long_help = "Control which logs are emitted if `--log` or `--perf-profile` are enabled.\nIf no filter is specified, Pavexc will default to `info,pavexc=trace`."
    )]
    pub log_filter: Option<String>,
    #[clap(
        long,
        env = "PAVEXC_PERF_PROFILE",
        help_heading = Some(INTROSPECTION_HEADING),
        hide_short_help = true,
        hide_env = true,
        long_help = "Pavexc will serialize to disk tracing information to profile command execution.\nThe file (`trace-[...].json`) can be opened using https://ui.perfetto.dev/ or in Google Chrome by visiting chrome://tracing.\nSet `PAVEXC_PERF_PROFILE=true` to enable this option using an environment variable."
    )]
    pub perf_profile: bool,
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

#[derive(Subcommand, Debug)]
enum Commands {
    /// Generate a server SDK crate according to an application blueprint.
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
        #[clap(long)]
        /// Verify that the generated server SDK is up-to-date.  
        /// If it isn't, `pavexc` will return an error without updating
        /// the server SDK code.
        check: bool,
        #[clap(long, env = "PAVEXC_DOCS_TOOLCHAIN", default_value = DEFAULT_DOCS_TOOLCHAIN)]
        /// The name of the `rustup` toolchain that `pavexc` will use to generate the JSON documentation
        /// for the crates in the dependency graph of this project.
        docs_toolchain: String,
    },
    /// Scaffold a new Pavex project at <PATH>.
    New {
        /// The path of the new directory that will contain the project files.
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
    /// Information about this version of `pavexc`.
    #[command(name = "self")]
    Self_ {
        #[clap(subcommand)]
        command: SelfCommands,
    },
}

#[derive(Subcommand, Debug)]
pub enum SelfCommands {
    Setup {
        #[clap(long, env = "PAVEXC_DOCS_TOOLCHAIN", default_value = DEFAULT_DOCS_TOOLCHAIN)]
        /// The name of the `rustup` toolchain that `pavexc` will use to generate the JSON documentation
        /// for the crates in the dependency graph of this project.
        ///
        /// It overrides the default toolchain associated with this version of `pavexc`.
        docs_toolchain: String,
    },
}

fn init_telemetry(
    log_filter: Option<String>,
    color: Color,
    console_logging: bool,
    profiling: bool,
) -> Option<FlushGuard> {
    let filter_layer = log_filter
        .map(|f| EnvFilter::try_new(f).expect("Invalid log filter configuration"))
        .unwrap_or_else(|| {
            EnvFilter::try_new("info,pavexc=debug").expect("Invalid log filter configuration")
        });
    let base = tracing_subscriber::registry();
    let mut chrome_guard = None;
    let trace_filename = format!(
        "./trace-pavexc-{}.json",
        std::time::SystemTime::UNIX_EPOCH
            .elapsed()
            .unwrap()
            .as_millis()
    );

    match console_logging {
        true => {
            let fmt_layer = tracing_subscriber::fmt::layer()
                .with_ansi(use_color_on_stderr(color))
                .with_file(false)
                .with_target(false)
                .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
                .with_timer(tracing_subscriber::fmt::time::uptime())
                .event_format(ReversedFull);
            let fmt_layer = Filtered {
                base: filter_layer,
                fields: BTreeMap::new(),
                layer: fmt_layer,
            };
            if profiling {
                let (chrome_layer, guard) = ChromeLayerBuilder::new()
                    .file(trace_filename)
                    .include_args(true)
                    .build();
                chrome_guard = Some(guard);
                base.with(fmt_layer).with(chrome_layer).init();
            } else {
                base.with(fmt_layer).init();
            }
        }
        false => {
            if profiling {
                let (chrome_layer, guard) = ChromeLayerBuilder::new()
                    .file(trace_filename)
                    .include_args(true)
                    .build();
                chrome_guard = Some(guard);
                base.with(chrome_layer).init()
            }
        }
    }
    chrome_guard
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

    better_panic::install();
    let _guard = init_telemetry(cli.log_filter.clone(), cli.color, cli.log, cli.perf_profile);

    tracing::trace!(cli = ?cli, "`pavexc` CLI options and flags");
    match cli.command {
        Commands::Generate {
            blueprint,
            diagnostics,
            output,
            check,
            docs_toolchain,
        } => generate(
            blueprint,
            docs_toolchain,
            diagnostics,
            output,
            cli.color,
            check,
        ),
        Commands::New { path, template } => scaffold_project(path, template),
        Commands::Self_ {
            command: SelfCommands::Setup { docs_toolchain },
        } => {
            let mut shell = Shell::new();
            let options = IfAutoinstallable::Autoinstall;
            let toolchain = RustupToolchain {
                name: docs_toolchain.clone(),
            };
            verify_installation(&mut shell, toolchain, options)?;
            let rustdoc_json = RustdocJson {
                toolchain: docs_toolchain,
            };
            verify_installation(&mut shell, rustdoc_json, options)?;
            Ok(ExitCode::SUCCESS)
        }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
/// The dependencies of this version of `pavexc`.
struct Deps {
    /// The toolchain that `pavexc` will use to generate the JSON docs for crates in the
    /// dependency tree of the current project.
    docs_toolchain: ToolchainInfo,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct ToolchainInfo {
    /// The name of the toolchain.
    ///
    /// This must be a valid identifier for `rustup toolchain install`.
    name: String,
}

#[tracing::instrument("Generate server sdk")]
fn generate(
    blueprint: PathBuf,
    docs_toolchain: String,
    diagnostics: Option<PathBuf>,
    output: PathBuf,
    color_profile: Color,
    check: bool,
) -> Result<ExitCode, Box<dyn std::error::Error>> {
    let blueprint = {
        let file = fs_err::OpenOptions::new().read(true).open(blueprint)?;
        ron::de::from_reader(&file)?
    };
    let mut reporter = DiagnosticReporter::new(color_profile);
    let (app, issues) = match App::build(blueprint, docs_toolchain) {
        Ok((a, issues)) => {
            for e in &issues {
                assert_eq!(e.severity(), Some(Severity::Warning));
            }
            (Some(a), issues)
        }
        Err(issues) => (None, issues),
    };

    for e in issues {
        reporter.print_report(&e);
    }

    let Some(app) = app else {
        return Ok(ExitCode::FAILURE);
    };
    if let Some(diagnostic_path) = diagnostics {
        app.diagnostic_representation()
            .persist_flat(&diagnostic_path)?;
    }
    let generated_app = app.codegen()?;
    let mut writer = if check {
        AppWriter::check_mode()
    } else {
        AppWriter::update_mode()
    };
    generated_app.persist(&output, &mut writer)?;
    if let Err(errors) = writer.verify() {
        for e in errors {
            reporter.print_report(&e);
        }
        return Ok(ExitCode::FAILURE);
    }
    Ok(ExitCode::SUCCESS)
}

/// The compiler may emit the same diagnostic more than once
/// (for a variety of reasons). We use this helper to dedup them.
struct DiagnosticReporter {
    already_emitted: HashSet<String>,
    use_color: bool,
}

impl DiagnosticReporter {
    fn new(color_profile: Color) -> Self {
        let use_color = use_color_on_stderr(color_profile);
        Self {
            already_emitted: Default::default(),
            use_color,
        }
    }
    fn print_report(&mut self, e: &miette::Report) {
        let formatted = format!("{e:?}");
        if self.already_emitted.contains(&formatted) {
            // Avoid printing the same diagnostic multiple times.
            return;
        }
        let prefix = match e.severity() {
            None | Some(Severity::Error) => {
                let mut p = "ERROR".to_string();
                if self.use_color {
                    p = p.bold().red().to_string();
                }
                p
            }
            Some(Severity::Warning) => {
                let mut p = "WARNING".to_string();
                if self.use_color {
                    p = p.bold().yellow().to_string();
                }
                p
            }
            _ => unreachable!(),
        };
        eprintln!("{prefix}: {formatted}");
        self.already_emitted.insert(formatted);
    }
}

fn use_color_on_stderr(color_profile: Color) -> bool {
    match color_profile {
        Color::Auto => supports_color::on(Stream::Stderr).is_some(),
        Color::Always => true,
        Color::Never => false,
    }
}

static TEMPLATE_DIR: include_dir::Dir = include_dir::include_dir!("$CARGO_MANIFEST_DIR/template");

fn scaffold_project(
    destination: PathBuf,
    template: TemplateName,
) -> Result<ExitCode, Box<dyn std::error::Error>> {
    let name = destination
        .file_name()
        .ok_or_else(|| {
            anyhow::anyhow!("Failed to derive a project name from the provided path")
        })?
        .to_str()
        .ok_or_else(|| {
            anyhow::anyhow!("The last segment of the provided path must be valid UTF8 to generate a valid project name")
        })?
        .to_string();

    let template_dir = tempfile::Builder::new()
        .prefix(&format!("pavex-template-{}", env!("VERGEN_GIT_SHA")))
        .tempdir()
        .context("Failed to create a temporary directory for Pavex's template")?;
    TEMPLATE_DIR
        .extract(&template_dir)
        .context("Failed to save Pavex's template to a temporary directory")?;

    let pavex_package_spec = std::env::var("CARGO_GENERATE_VALUE_PAVEX_PACKAGE_SPEC")
        .unwrap_or_else(|_| format!("version = \"{}\"", env!("CARGO_PKG_VERSION")));
    let pavex_tracing_package_spec =
        std::env::var("CARGO_GENERATE_VALUE_PAVEX_TRACING_PACKAGE_SPEC")
            .unwrap_or_else(|_| format!("version = \"{}\"", env!("CARGO_PKG_VERSION")));
    let pavex_cli_client_package_spec =
        std::env::var("CARGO_GENERATE_VALUE_PAVEX_CLI_CLIENT_PACKAGE_SPEC")
            .unwrap_or_else(|_| format!("version = \"{}\"", env!("CARGO_PKG_VERSION")));

    let add_greet_route = template == TemplateName::Api;

    let mut define = HashMap::new();
    define.insert(
        "pavex_package_spec".to_string(),
        Value::scalar(pavex_package_spec.clone()),
    );
    define.insert(
        "pavex_cli_client_package_spec".to_string(),
        Value::scalar(pavex_cli_client_package_spec.clone()),
    );
    define.insert(
        "pavex_tracing_package_spec".to_string(),
        Value::scalar(pavex_tracing_package_spec.clone()),
    );
    define.insert("greet_route".to_string(), Value::scalar(add_greet_route));

    let destination = {
        use path_absolutize::Absolutize;

        destination
            .absolutize()
            .map(|p| p.to_path_buf())
            .context("Failed to convert the provided path to an absolute path")?
    };
    let destination_parent = destination
        .parent()
        .context("Failed to derive the parent directory of the provided path")?;
    let mut ignore = vec!["target/".into(), "Cargo.lock".into(), ".idea".into()];
    if !add_greet_route {
        ignore.push("app/src/routes/greet.rs".into());
    }

    let generate_args = GenerateArgs {
        template_dir: template_dir.path().to_path_buf(),
        destination: destination_parent.to_path_buf(),
        name: name.clone(),
        define,
        ignore: Some(ignore),
        overwrite: false,
        verbose: false,
    };
    eprintln!(
        "Generating a new Pavex project in {} with {name}",
        destination.display()
    );
    generate_from_path::generate(generate_args)
        .context("Failed to scaffold the project from Pavex's default template")?;
    // We don't care if this fails, as it's just a nice-to-have.
    if let Err(e) = cargo_fmt(&destination) {
        tracing::warn!(error.msg = %e, error.details = ?e, "Failed to format the generated project");
    }
    Ok(ExitCode::SUCCESS)
}

/// Use `cargo` to format the generated project.
fn cargo_fmt(project_dir: &Path) -> anyhow::Result<()> {
    let output = std::process::Command::new("cargo")
        .arg("fmt")
        .current_dir(project_dir)
        .output()?;
    if !output.status.success() {
        anyhow::bail!(
            "Failed to format the generated project at {}: {}",
            project_dir.display(),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}
