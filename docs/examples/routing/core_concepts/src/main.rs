use std::error::Error;

use cargo_px_env::generated_pkg_manifest_path;
use pavex_cli_client::Client;

use core_concepts::blueprint;

fn main() -> Result<(), Box<dyn Error>> {
    let generated_dir = generated_pkg_manifest_path()?.parent().unwrap().into();
    Client::new()
        .generate(blueprint(), generated_dir)
        .execute()?;
    Ok(())
}
