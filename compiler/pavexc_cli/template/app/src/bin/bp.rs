use app::blueprint;
use cargo_px_env::generated_pkg_manifest_path;
use pavex_cli_client::Client;
use std::env::args;
use std::error::Error;

/// Generate the `server_sdk` crate using Pavex's CLI.
///
/// Pavex will automatically wire all our routes, constructors and error handlers
/// into a "server SDK" that can be used by the final API server binary to launch
/// the application.
///
/// If `--check` is passed as an argument, it only verifies that the server SDK
/// crate is up-to-date. An error is returned if it isn't.
fn main() -> Result<(), Box<dyn Error>> {
    let generated_dir = generated_pkg_manifest_path()?.parent().unwrap().into();
    let mut cmd = Client::new().generate(blueprint(), generated_dir);
    if args().any(|arg| arg == "--check") {
        cmd = cmd.check()
    };
    if let Err(e) = cmd.execute() {
        eprintln!("{e}");
        std::process::exit(1);
    }
    Ok(())
}
