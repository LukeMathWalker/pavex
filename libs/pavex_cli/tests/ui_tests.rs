use pavex_test_runner::run_tests;
use std::path::PathBuf;
use std::process::ExitCode;
use std::str::FromStr;
use tracing_chrome::{ChromeLayerBuilder, FlushGuard};
use tracing_subscriber::Registry;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

fn main() -> Result<ExitCode, Box<dyn std::error::Error>> {
    let _guard = if std::env::var("PAVEX_TEST_LOG").is_ok_and(|s| s == "true") {
        init_telemetry()
    } else {
        None
    };

    let manifest_dir = PathBuf::from_str(env!("CARGO_MANIFEST_DIR")).unwrap();
    let test_runtime_folder = manifest_dir.parent().unwrap().join("ui_tests");
    let pavexc_cli_path = get_pavexc_cli_path()?;
    let pavex_cli_path = get_pavex_cli_path()?;
    let code = run_tests(pavex_cli_path, pavexc_cli_path, test_runtime_folder)?.exit_code();
    Ok(code)
}

fn init_telemetry() -> Option<FlushGuard> {
    let profiling = std::env::var("PAVEX_TEST_PROFILING").is_ok_and(|s| s == "true");
    let mut chrome_guard = None;
    let base = Registry::default().with(tracing_subscriber::fmt::layer());
    if profiling {
        let trace_filename = format!(
            "./trace-pavex-cli-ui-tests-{}.json",
            std::time::SystemTime::UNIX_EPOCH
                .elapsed()
                .unwrap()
                .as_millis()
        );
        let (chrome_layer, guard) = ChromeLayerBuilder::new()
            .file(trace_filename)
            .include_args(true)
            .build();
        chrome_guard = Some(guard);
        base.with(chrome_layer).init();
    } else {
        base.init();
    }
    chrome_guard
}

fn get_pavex_cli_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Ok(path) = std::env::var("PAVEX_TEST_CLI_PATH") {
        return Ok(PathBuf::from_str(&path)?);
    }

    let profile = std::env::var("PAVEX_TEST_CLI_PROFILE").unwrap_or("debug".to_string());
    let manifest_dir = PathBuf::from_str(env!("CARGO_MANIFEST_DIR"))?;
    let target_dir = manifest_dir.parent().unwrap().join("target");
    Ok(target_dir.join(profile).join("pavex"))
}

fn get_pavexc_cli_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Ok(path) = std::env::var("PAVEXC_TEST_CLI_PATH") {
        return Ok(PathBuf::from_str(&path)?);
    }

    let profile = std::env::var("PAVEXC_TEST_CLI_PROFILE").unwrap_or("debug".to_string());
    let manifest_dir = PathBuf::from_str(env!("CARGO_MANIFEST_DIR"))?;
    let target_dir = manifest_dir.parent().unwrap().join("target");
    Ok(target_dir.join(profile).join("pavexc"))
}
