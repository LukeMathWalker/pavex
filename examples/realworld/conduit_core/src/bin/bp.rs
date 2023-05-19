use cargo_px_env::generated_pkg_manifest_path;
use conduit_core::api_blueprint;
use pavex_cli_client::Client;
use std::error::Error;

/// Generate the `api_server_sdk` crate using `pavex`'s CLI.
/// 
/// `pavex` will automatically wire all our routes, constructors and error handlers
/// into the a "server SDK" that can be used by the final API server binary to launch
/// the application.
fn main() -> Result<(), Box<dyn Error>> {
    let generated_dir = generated_pkg_manifest_path()?.parent().unwrap().into();
    let blueprint = api_blueprint();
    Client::new()
        .pavex_cli_path("../../libs/target/release/pavex_cli".into())
        .generate(blueprint, generated_dir)
        .execute()?;
    Ok(())
}
