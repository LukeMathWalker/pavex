use anyhow::Context;
use cargo_like_utils::shell::Shell;
use std::io::{ErrorKind, IsTerminal};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::Parser;
use jsonwebtoken::jwk::JwkSet;
use owo_colors::OwoColorize;
use pavex_cli::activation::{
    check_activation, check_activation_if_necessary, check_activation_with_key, CliTokenError,
};
use pavex_cli::cargo_install::{cargo_install, GitSourceRevision, Source};
use pavex_cli::cli_kind::CliKind;
use pavex_cli::command::{Cli, Color, Command, SelfCommands};
use pavex_cli::dependencies::installers;
use pavex_cli::dependencies::installers::{CargoPx, NightlyToolchain, RustdocJson, Rustup};
use pavex_cli::locator::PavexLocator;
use pavex_cli::package_graph::compute_package_graph;
use pavex_cli::pavexc::{get_or_install_from_graph, get_or_install_from_version};
use pavex_cli::prebuilt::download_prebuilt;
use pavex_cli::state::State;
use pavex_cli::user_input::{confirm, mandatory_question};
use pavex_cli::utils;
use pavex_cli::version::latest_released_version;
use pavexc_cli_client::commands::generate::{BlueprintArgument, GenerateError};
use pavexc_cli_client::commands::new::NewError;
use pavexc_cli_client::Client;
use redact::Secret;
use semver::Version;
use supports_color::Stream;
use tracing_chrome::{ChromeLayerBuilder, FlushGuard};
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

static PAVEX_CACHED_KEYSET: &str = include_str!("../jwks.json");

fn main() -> Result<ExitCode, miette::Error> {
    let cli = Cli::parse();
    init_miette_hook(&cli);
    let _guard = cli.debug.then(init_telemetry);

    let mut client = Client::new().color(cli.color.into());
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
    let key_set: JwkSet =
        serde_json::from_str(PAVEX_CACHED_KEYSET).expect("Failed to parse the cached JWKS");

    check_activation_if_necessary(&cli.command, &locator, &mut shell, &key_set)
        .map_err(utils::anyhow2miette)?;
    match cli.command {
        Command::Generate {
            blueprint,
            diagnostics,
            check,
            output,
        } => generate(
            &mut shell,
            client,
            &locator,
            blueprint,
            diagnostics,
            output,
            check,
        ),
        Command::New { path } => scaffold_project(client, &locator, &mut shell, path),
        Command::Self_ { command } => {
            // You should always be able to run `self` commands, even if Pavex has
            // not been activated yet.
            match command {
                SelfCommands::Update => update(&mut shell),
                SelfCommands::Uninstall { y } => uninstall(&mut shell, !y, locator),
                SelfCommands::Activate { key } => activate(
                    &mut shell,
                    cli.color,
                    &locator,
                    key.map(|k| k.into_inner()),
                    &key_set,
                ),
                SelfCommands::Setup => setup(cli.debug, &mut shell, cli.color, &locator, &key_set),
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
    check: bool,
) -> Result<ExitCode, anyhow::Error> {
    let pavexc_cli_path = if let Some(pavexc_override) = pavex_cli::env::pavexc_override() {
        pavexc_override
    } else {
        // Match the version of the `pavexc` binary with the version of the `pavex` library
        // crate used in the current workspace.
        let package_graph = compute_package_graph()
            .context("Failed to compute package graph for the current workspace")?;
        get_or_install_from_graph(shell, locator, &package_graph)?
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

    shell.status(
        "Creating",
        format!("the new project in `{}`", path.display()),
    )?;
    match client.new_command(path.clone()).execute() {
        Ok(()) => {
            shell.status(
                "Created",
                format!("the new project in `{}`", path.display()),
            )?;
            Ok(ExitCode::SUCCESS)
        }
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
        let continue_ = confirm("\nDo you wish to continue?", false)?;
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

    shell.status(
        "Updated",
        format!("to {latest_version}, the most recent version"),
    )?;

    Ok(ExitCode::SUCCESS)
}

#[tracing::instrument("Setup Pavex", skip_all)]
fn setup(
    debug: bool,
    shell: &mut Shell,
    color: Color,
    locator: &PavexLocator,
    key_set: &JwkSet,
) -> Result<ExitCode, anyhow::Error> {
    fn _setup(
        shell: &mut Shell,
        color: Color,
        locator: &PavexLocator,
        key_set: &JwkSet,
    ) -> Result<(), anyhow::Error> {
        installers::verify_installation::<Rustup>(shell)?;
        installers::verify_installation::<NightlyToolchain>(shell)?;
        installers::verify_installation::<RustdocJson>(shell)?;
        installers::verify_installation::<CargoPx>(shell)?;

        let _ = shell.status("Checking", "if Pavex has been activated");
        if check_activation(locator, shell, key_set).is_err() {
            let _ = shell.status_with_color(
                "Missing",
                "Pavex has not been activated yet",
                &cargo_like_utils::shell::style::ERROR,
            );
            if std::io::stdout().is_terminal() {
                activate(shell, color, locator, None, key_set)?;
            } else {
                let _ = shell.note(
                    "Invoke\n\n    \
                    pavex self activate\n\n\
                    to activate your Pavex installation.",
                );
            }
        } else {
            let _ = shell.status("Success", "Pavex has already been activated");
        }

        Ok(())
    }

    match _setup(shell, color, locator, key_set) {
        Ok(()) => Ok(ExitCode::SUCCESS),
        Err(e) => {
            if debug {
                Err(e)
            } else {
                let _ = shell.note(
                    "Once you've installed what's missing, re-run `pavex self setup` \
                    to verify everything is working as expected.",
                );
                Ok(ExitCode::FAILURE)
            }
        }
    }
}

#[tracing::instrument("Activate Pavex", skip_all)]
fn activate(
    shell: &mut Shell,
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
            let mut k: Option<Secret<String>> = None;
            'outer: while k.is_none() {
                let question = if use_color_on_stdout(color) {
                    format!(
                        "Welcome to Pavex's beta program! Please enter your {}.\n{}",
                        "activation key".bold().green(),
                        "You can find the activation key for the beta program in the #activation \
                        channel of Pavex's Discord server.\n\
                        You can join the beta program by visiting https://pavex.dev\n"
                            .dimmed()
                    )
                } else {
                    "Welcome to Pavex's beta program! Please enter your activation key.\n\
                        You can find the activation key for the beta program in the #activation \
                        channel of Pavex's Discord server.\n\
                        You can join the beta program by visiting https://pavex.dev\n"
                        .to_string()
                };
                let attempt = Secret::new(
                    mandatory_question(&question).context("Failed to read activation key")?,
                );
                match check_activation_with_key(locator, attempt.clone(), key_set) {
                    Ok(_) => {
                        k = Some(attempt);
                        break 'outer;
                    }
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
                                Please try again."),
                                color,
                            );
                        }
                    },
                }
                eprintln!();
            }
            k.unwrap()
        }
        Some(k) => {
            if check_activation_with_key(locator, k.clone(), key_set).is_err() {
                return Err(anyhow::anyhow!(
                    "The activation key you provided is not valid"
                ));
            }
            k
        }
    };
    state
        .set_activation_key(shell, key)
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
