use app_blueprint::app_blueprint;
use cargo_px_env::generated_pkg_manifest_path;
use pavex_cli_client::Client;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let generated_dir = generated_pkg_manifest_path()?.parent().unwrap().into();
    let blueprint = app_blueprint();
    Client::new()
        .pavex_cli_path("../../libs/target/release/pavex_cli".into())
        .generate(blueprint, generated_dir)
        .execute()?;
    Ok(())
}
