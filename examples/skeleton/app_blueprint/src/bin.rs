use std::error::Error;
use std::path::PathBuf;

use app_blueprint::app_blueprint;

fn main() -> Result<(), Box<dyn Error>> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root_dir = manifest_dir.parent().unwrap();

    let bp_path = workspace_root_dir.join("blueprint.ron");
    app_blueprint().persist(&bp_path)?;

    std::process::Command::new("../../libs/target/debug/pavex_cli")
        .arg("generate")
        .arg("-b")
        .arg(bp_path)
        .arg("-o")
        .arg("generated_app")
        .status()?;
    Ok(())
}
