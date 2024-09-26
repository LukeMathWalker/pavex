//! This code is generated by `pavex_test_runner`,
//! Do NOT modify it manually.
use app_c1ac8ad4::blueprint;
use pavex_cli_client::{Client, config::Color};
use pavex_cli_client::commands::generate::GenerateError;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ui_test_dir: std::path::PathBuf = std::env::var("UI_TEST_DIR").unwrap().into();
    let outcome = Client::new()
        .color(Color::Always)
        .pavex_cli_path(std::env::var("PAVEX_TEST_CLI_PATH").unwrap().into())
        .generate(blueprint(), ui_test_dir.join("generated_app"))
        .diagnostics_path("diagnostics.dot".into())
        .execute();
    match outcome {
        Ok(_) => {},
        Err(GenerateError::NonZeroExitCode(_)) => { std::process::exit(1); }
        Err(e) => {
            eprintln!("Failed to invoke `pavex generate`.\n{:?}", e);
            std::process::exit(1);
        }
    }
    Ok(())
}
