use std::error::Error;

use cargo_px_env::generated_pkg_manifest_path;
use in_memory_sessions::blueprint;
use pavex_cli_client::Client;

fn main() -> Result<(), Box<dyn Error>> {
    let generated_dir = generated_pkg_manifest_path()?.parent().unwrap().into();
    Client::new()
        .generate(blueprint(), generated_dir)
        .execute()?;
    Ok(())
}
