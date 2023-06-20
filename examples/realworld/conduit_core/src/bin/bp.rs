use cargo_px_env::generated_pkg_manifest_path;
use conduit_core::blueprint;
use pavex_cli_client::Client;
use std::error::Error;

/// Generate the `api_server_sdk` crate using Pavex's CLI.
///
/// Pavex will automatically wire all our routes, constructors and error handlers
/// into the a "server SDK" that can be used by the final API server binary to launch
/// the application.
fn main() -> Result<(), Box<dyn Error>> {
    let generated_dir = generated_pkg_manifest_path()?.parent().unwrap().into();
    Client::new()
        // This customization is only needed because the example lives in the same
        // repository of Pavex itself. In a real-world scenario, you would just
        // use the binary path.
        .pavex_cli_path("../../libs/target/release/pavex_cli".into())
        .generate(blueprint(), generated_dir)
        .execute()?;
    Ok(())
}
