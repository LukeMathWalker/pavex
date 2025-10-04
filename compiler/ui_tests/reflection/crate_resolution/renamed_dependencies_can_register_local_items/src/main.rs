use app_11a9a819::blueprint;
use pavex_cli_client::commands::generate::GenerateError;
use pavex_cli_client::{config::Color, Client};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let outcome = Client::new()
        .color(Color::Always)
        .pavex_cli_path(std::env::var("PAVEX_TEST_CLI_PATH").unwrap().into())
        .generate(blueprint(), "generated_app".into())
        .diagnostics_path("diagnostics.dot".into())
        .execute();
    match outcome {
        Ok(_) => {}
        Err(GenerateError::NonZeroExitCode(_)) => {
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("Failed to invoke `pavex generate`.\n{:?}", e);
            std::process::exit(1);
        }
    }
    Ok(())
}
