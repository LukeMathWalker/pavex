use anyhow::Context;
use cargo_like_utils::shell::Shell;
use pavex_cli_diagnostic::AnyhowBridge;
use pavex_cli_shell::{SHELL, ShellExt, try_init_shell};
use std::io::{ErrorKind, IsTerminal};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use tracing_log_error::log_error;

use clap::Parser;
use jsonwebtoken::jwk::JwkSet;
use owo_colors::OwoColorize;
use pavex_cli::activation::{
    CliTokenError, background_token_refresh, check_activation, exchange_wizard_key,
    get_activation_key, get_activation_key_if_necessary,
};
use pavex_cli::cargo_install::{GitSourceRevision, Source, cargo_install};
use pavex_cli::cli_kind::CliKind;
use pavex_cli::command::{Cli, Color, Command, SelfCommands};
use pavex_cli::locator::PavexLocator;
use pavex_cli::package_graph::compute_package_graph;
use pavex_cli::pavexc::{get_or_install_from_graph, get_or_install_from_version};
use pavex_cli::prebuilt::download_prebuilt;
use pavex_cli::state::State;
use pavex_cli::user_input::{confirm, mandatory_question};
use pavex_cli::version::latest_released_version;
use pavex_cli_deps::{CargoPx, IfAutoinstallable, Rustup, verify_installation};
use pavexc_cli_client::Client;
use pavexc_cli_client::commands::generate::{BlueprintArgument, GenerateError};
use pavexc_cli_client::commands::new::NewError;
use pavexc_cli_client::commands::new::TemplateName;
use redact::Secret;
use semver::Version;
use supports_color::Stream;
use tracing_chrome::{ChromeLayerBuilder, FlushGuard};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

static PAVEX_CACHED_KEYSET: &str = include_str!("../jwks.json");

fn main() -> ExitCode {
    let cli = Cli::parse();
    init_miette_hook(&cli);
    let _guard = init_telemetry(cli.log_filter.clone(), cli.color, cli.log, cli.perf_profile);
    let code = match _main(cli) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("{e:?}");
            ExitCode::FAILURE
        }
    };
    if code != ExitCode::SUCCESS {
        SHELL.note("Rerun with `PAVEX_DEBUG=true` to display more error details.");
    }
    code
}

fn _main(cli: Cli) -> Result<ExitCode, miette::Error> {
    init_shell(cli.color).map_err(|e| e.into_miette())?;

    let client = pavexc_client(&cli);
    let system_home_dir = xdg_home::home_dir().ok_or_else(|| {
        miette::miette!("Failed to get the system home directory from the environment")
    })?;
    let locator = PavexLocator::new(&system_home_dir);
    let key_set: JwkSet =
        serde_json::from_str(PAVEX_CACHED_KEYSET).expect("Failed to parse the cached JWKS");

    if let Some(activation_key) =
        get_activation_key_if_necessary(&cli.command, &locator).map_err(|e| e.into_miette())?
    {
        let claims = check_activation(&locator, activation_key.clone(), &key_set)
            .context("Failed to check Pavex activation")
            .map_err(|e| e.into_miette())?;
        background_token_refresh(&claims, &key_set, activation_key, &locator);
    }
    match cli.command {
        Command::Generate {
            blueprint,
            diagnostics,
            check,
            output,
        } => generate(client, &locator, blueprint, diagnostics, output, check)
            .map_err(|e| e.into_miette().into()),
        Command::New { path, template } => {
            scaffold_project(client, &locator, path, template).map_err(|e| e.into_miette().into())
        }
        Command::Self_ { command } => {
            // You should always be able to run `self` commands, even if Pavex has
            // not been activated yet.
            match command {
                SelfCommands::Update => update().map_err(|e| e.into_miette().into()),
                SelfCommands::Uninstall { y } => {
                    uninstall(!y, locator).map_err(|e| e.into_miette().into())
                }
                SelfCommands::Activate { key } => {
                    activate(cli.color, &locator, key.map(|k| k.into_inner()), &key_set)
                        .map_err(|e| e.into_miette().into())
                }
                SelfCommands::Setup {
                    wizard_key,
                    skip_activation,
                } => setup(cli.color, &locator, &key_set, wizard_key, skip_activation)
                    .map(|_| ExitCode::SUCCESS),
            }
        }
    }
}

fn init_shell(color: Color) -> Result<(), anyhow::Error> {
    let mut shell = Shell::new();
    shell
        .set_color_choice(Some(match color {
            Color::Auto => "auto",
            Color::Always => "always",
            Color::Never => "never",
        }))
        .context("Failed to configure shell output")?;
    try_init_shell(shell);
    Ok(())
}

/// Propagate introspection options from `pavex` to pavexc`.
fn pavexc_client(cli: &Cli) -> Client {
    let mut client = Client::new().color(cli.color.into());
    if cli.debug {
        client = client.debug();
    } else {
        client = client.no_debug();
    }
    if cli.log {
        client = client.log();
    } else {
        client = client.no_log();
    }
    if cli.perf_profile {
        client = client.perf_profile();
    } else {
        client = client.no_perf_profile();
    }
    if let Some(log_filter) = &cli.log_filter {
        client = client.log_filter(log_filter.to_owned());
    }
    client
}

#[tracing::instrument("Generate server sdk", skip(client, locator))]
fn generate(
    mut client: Client,
    locator: &PavexLocator,
    blueprint: PathBuf,
    diagnostics: Option<PathBuf>,
    output: PathBuf,
    check: bool,
) -> Result<ExitCode, anyhow::Error> {
    let pavexc_cli_path = if let Some(pavexc_override) = pavex_cli::env::pavexc_override() {
        pavexc_override
    } else {
        // Match the version of the `pavexc` binary with the version of the `pavex` library
        // crate used in the current workspace.
        let package_graph = compute_package_graph()
            .context("Failed to compute package graph for the current workspace")?;
        get_or_install_from_graph(locator, &package_graph)?
    };
    client = client.pavexc_cli_path(pavexc_cli_path);

    let blueprint = BlueprintArgument::Path(blueprint);
    let mut cmd = client.generate(blueprint, output);
    if let Some(diagnostics) = diagnostics {
        cmd = cmd.diagnostics_path(diagnostics)
    };
    if check {
        cmd = cmd.check();
    }

    match cmd.execute() {
        Ok(()) => Ok(ExitCode::SUCCESS),
        Err(GenerateError::NonZeroExitCode(e)) => Ok(ExitCode::from(e.code as u8)),
        Err(e) => Err(e.into()),
    }
}

#[tracing::instrument("Scaffold new project", skip(client, locator))]
fn scaffold_project(
    mut client: Client,
    locator: &PavexLocator,
    path: PathBuf,
    template: TemplateName,
) -> Result<ExitCode, anyhow::Error> {
    let pavexc_cli_path = if let Some(pavexc_override) = pavex_cli::env::pavexc_override() {
        pavexc_override
    } else {
        let version = State::new(locator)
            .get_current_toolchain()
            .context("Failed to get the current toolchain")?;
        get_or_install_from_version(locator, &version)
            .context("Failed to get or install the `pavexc` binary")?
    };

    client = client.pavexc_cli_path(pavexc_cli_path);

    SHELL.status(
        "Creating",
        format!("the new project in `{}`", path.display()),
    );
    match client
        .new_command(path.clone())
        .template(template)
        .execute()
    {
        Ok(()) => {
            SHELL.status(
                "Created",
                format!("the new project in `{}`", path.display()),
            );
            Ok(ExitCode::SUCCESS)
        }
        Err(NewError::NonZeroExitCode(e)) => Ok(ExitCode::from(e.code as u8)),
        Err(e) => Err(e.into()),
    }
}

#[tracing::instrument("Uninstall Pavex CLI", skip(locator))]
fn uninstall(must_prompt_user: bool, locator: PavexLocator) -> Result<ExitCode, anyhow::Error> {
    SHELL.status("Thanks", "for hacking with Pavex!");
    if must_prompt_user {
        SHELL.warn(
            "This process will uninstall Pavex and all its associated data from your system.",
        );
        let continue_ = confirm("\nDo you wish to continue?", false)?;
        if !continue_ {
            SHELL.status("Abort", "Uninstalling Pavex CLI");
            return Ok(ExitCode::SUCCESS);
        }
    }

    SHELL.status("Uninstalling", "Pavex");
    if let Err(e) = fs_err::remove_dir_all(locator.root_dir()) {
        if ErrorKind::NotFound != e.kind() {
            Err(e).context("Failed to remove Pavex data")?;
        }
    }
    self_replace::self_delete().context("Failed to delete the current Pavex CLI binary")?;
    SHELL.status("Uninstalled", "Pavex");

    Ok(ExitCode::SUCCESS)
}

#[tracing::instrument("Update Pavex CLI", skip_all)]
fn update() -> Result<ExitCode, anyhow::Error> {
    SHELL.status("Checking", "for updates to Pavex CLI");
    let latest_version = latest_released_version()?;
    let current_version = pavex_cli::env::version();
    if latest_version <= current_version {
        SHELL.status(
            "Up to date",
            format!("{current_version} is the most recent version"),
        );
        return Ok(ExitCode::SUCCESS);
    }

    SHELL.status(
        "Update available",
        format!("You're running {current_version}, but {latest_version} is available"),
    );

    let new_cli_path = tempfile::NamedTempFile::new()
        .context("Failed to create a temporary file to download the new Pavex CLI binary")?;
    download_or_compile(CliKind::Pavex, &latest_version, new_cli_path.path())?;
    self_replace::self_replace(new_cli_path.path())
        .context("Failed to replace the current Pavex CLI with the newly downloaded version")?;

    SHELL.status(
        "Updated",
        format!("to {latest_version}, the most recent version"),
    );

    Ok(ExitCode::SUCCESS)
}

#[tracing::instrument("Setup Pavex", skip_all)]
fn setup(
    color: Color,
    locator: &PavexLocator,
    key_set: &JwkSet,
    wizard_key: Option<Secret<String>>,
    skip_activation: bool,
) -> Result<(), miette::Error> {
    if skip_activation {
        SHELL.status("Skipping", "Pavex activation");
    } else {
        SHELL.status("Checking", "if Pavex has been activated on your machine");
        let must_activate = match get_activation_key(locator) {
            Ok(key) => check_activation(locator, key.clone(), key_set).is_err(),
            Err(_) => true,
        };
        if must_activate {
            match wizard_key {
                Some(key) => {
                    exchange_wizard_key(locator, key)?;
                    SHELL.status("Success", "Pavex has been activated on your machine");
                }
                None => {
                    SHELL.status_with_color(
                        "Inactive",
                        "Pavex has not been activated yet",
                        &cargo_like_utils::shell::style::ERROR,
                    );
                    activate(color, locator, None, key_set).map_err(|e| e.into_miette())?;
                }
            }
        } else {
            SHELL.status(
                "Success",
                "Pavex has already been activated on your machine",
            );
        }
    }

    let options = IfAutoinstallable::PromptForConfirmation;
    verify_installation(Rustup, options)?;
    verify_installation(CargoPx, options)?;

    Ok(())
}

#[tracing::instrument("Activate Pavex", skip_all)]
fn activate(
    color: Color,
    locator: &PavexLocator,
    activation_key: Option<Secret<String>>,
    key_set: &JwkSet,
) -> Result<ExitCode, anyhow::Error> {
    fn print_error(msg: &str, color: Color) {
        if use_color_on_stderr(color) {
            eprintln!("{}: {msg}", "ERROR".bold().red());
        } else {
            eprintln!("ERROR: {msg}");
        }
    }

    let state = State::new(locator);
    let key = match activation_key {
        None => {
            let stdout = std::io::stdout();
            if !stdout.is_terminal() {
                return Err(anyhow::anyhow!(
                    "The current terminal does not support interactive prompts. If you want to activate \
                    Pavex in a non-interactive environment, please provide the activation key using one of \
                    the following methods:\n\
                    - Pass the activation key as standard input to the `pavex self activate` command (`echo \"<your-key>\" | pavex self activate` on Unix platforms)\n\
                    - Pass the activation key as an argument to the `pavex self activate` command (`pavex self activate \"<your-key>\"`)\n\
                    - Set the `PAVEX_ACTIVATION_KEY` environment variable"
                ));
            }

            println!();
            let k: Secret<String> = 'outer: loop {
                let question = if use_color_on_stdout(color) {
                    format!(
                        "Welcome to Pavex's beta program! Please enter your {}.\n{}",
                        "activation key".bold().green(),
                        "You can provision an activation key at https://console.pavex.dev\n"
                            .dimmed()
                    )
                } else {
                    "Welcome to Pavex's beta program! Please enter your activation key.\n\
                    You can provision an activation key at https://console.pavex.dev\n"
                        .to_string()
                };
                let attempt = Secret::new(
                    mandatory_question(&question).context("Failed to read activation key")?,
                );
                match check_activation(locator, attempt.clone(), key_set) {
                    Ok(_) => break 'outer attempt,
                    Err(e) => match e {
                        CliTokenError::ActivationKey(_) => {
                            print_error(
                                "The activation key you provided is not valid. Please try again.",
                                color,
                            );
                        }
                        CliTokenError::RpcError(e) => {
                            print_error(
                                &format!(
                                    "Something went wrong when I tried to verify your activation key against Pavex's API:\n\
                                {e:?}\n\
                                Please try again."
                                ),
                                color,
                            );
                        }
                    },
                }
                eprintln!();
            };
            k
        }
        Some(k) => {
            if check_activation(locator, k.clone(), key_set).is_err() {
                return Err(anyhow::anyhow!(
                    "The activation key you provided is not valid"
                ));
            }
            k
        }
    };
    state
        .set_activation_key(key)
        .context("Failed to set the activation key")?;

    if use_color_on_stdout(color) {
        println!(
            "{} ✅\n{}",
            "The key is valid".bold().green(),
            "Enjoy Pavex!".white()
        )
    } else {
        println!("The key is valid. Enjoy Pavex!")
    }

    Ok(ExitCode::SUCCESS)
}

fn download_or_compile(
    kind: CliKind,
    version: &Version,
    destination: &Path,
) -> Result<(), anyhow::Error> {
    SHELL.status(
        "Downloading",
        format!("prebuilt `{}@{version}` binary", kind.binary_target_name()),
    );
    match download_prebuilt(destination, kind, version) {
        Ok(_) => {
            SHELL.status(
                "Downloaded",
                format!("prebuilt `{}@{version}` binary", kind.binary_target_name()),
            );
            return Ok(());
        }
        Err(e) => {
            SHELL.warn(format!(
                "Download failed: {e}.\nI'll try compiling from source instead."
            ));
            log_error!(
                e, level: tracing::Level::WARN,
                "Failed to download prebuilt `{}` binary. I'll try to build it from source instead.", kind.binary_target_name()
            );
        }
    }

    SHELL.status(
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
    SHELL.status(
        "Compiled",
        format!("`{}@{version}` from source", kind.package_name()),
    );
    Ok(())
}

fn use_color_on_stdout(color_profile: Color) -> bool {
    match color_profile {
        Color::Auto => supports_color::on(Stream::Stdout).is_some(),
        Color::Always => true,
        Color::Never => false,
    }
}

fn use_color_on_stderr(color_profile: Color) -> bool {
    match color_profile {
        Color::Auto => supports_color::on(Stream::Stderr).is_some(),
        Color::Always => true,
        Color::Never => false,
    }
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
            EnvFilter::try_new("info,pavex=trace").expect("Invalid log filter configuration")
        });
    let base = tracing_subscriber::registry().with(filter_layer);
    let mut chrome_guard = None;
    let trace_filename = format!(
        "./trace-pavex-{}.json",
        std::time::SystemTime::UNIX_EPOCH
            .elapsed()
            .unwrap()
            .as_millis()
    );

    match console_logging {
        true => {
            let fmt_layer = tracing_subscriber::fmt::layer()
                .with_file(false)
                .with_ansi(use_color_on_stderr(color))
                .with_target(false)
                .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
                .with_timer(tracing_subscriber::fmt::time::uptime());
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

fn init_miette_hook(cli: &Cli) {
    let is_debug = cli.debug;
    let color = cli.color;
    miette::set_hook(Box::new(move |_| {
        let handler = pavex_miette::PavexMietteHandlerOpts::new();
        let mut handler = if is_debug {
            handler.with_cause_chain()
        } else {
            handler.without_cause_chain()
        };
        if let Some(width) = pavex_cli::env::tty_width() {
            handler = handler.width(width);
        }
        match color {
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
    .expect("Failed to initialize `miette`'s error hook");
}
