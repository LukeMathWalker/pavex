use std::error::Error;
use std::path::PathBuf;
use std::str::FromStr;

use app_blueprint::app_blueprint;

fn main() -> Result<(), Box<dyn Error>> {
    let path = PathBuf::from_str("blueprint.ron")?;
    app_blueprint().persist(&path)?;

    std::process::Command::new("../../target/debug/pavex_cli")
        .arg("generate")
        .arg("-b")
        .arg(path)
        .arg("-o")
        .arg("examples/generated_app")
        .status()?;
    Ok(())
}
