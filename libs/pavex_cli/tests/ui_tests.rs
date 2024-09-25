use pavex_test_runner::run_tests;
use std::path::PathBuf;
use std::str::FromStr;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = PathBuf::from_str(env!("CARGO_MANIFEST_DIR")).unwrap();
    let test_runtime_folder = manifest_dir.parent().unwrap().join("ui_tests");
    let pavexc_cli_path = get_pavexc_cli_path()?;
    let pavex_cli_path = get_pavex_cli_path()?;
    run_tests(pavex_cli_path, pavexc_cli_path, test_runtime_folder)?.exit();
}

fn get_pavex_cli_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Ok(path) = std::env::var("PAVEX_TEST_CLI_PATH") {
        return Ok(PathBuf::from_str(&path)?);
    }

    let profile = std::env::var("PAVEX_TEST_CLI_PROFILE").unwrap_or("debug".to_string());
    let manifest_dir = PathBuf::from_str(env!("CARGO_MANIFEST_DIR"))?;
    let target_dir = manifest_dir.parent().unwrap().join("target");
    Ok(target_dir.join(profile).join("pavex"))
}

fn get_pavexc_cli_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Ok(path) = std::env::var("PAVEXC_TEST_CLI_PATH") {
        return Ok(PathBuf::from_str(&path)?);
    }

    let profile = std::env::var("PAVEXC_TEST_CLI_PROFILE").unwrap_or("debug".to_string());
    let manifest_dir = PathBuf::from_str(env!("CARGO_MANIFEST_DIR"))?;
    let target_dir = manifest_dir.parent().unwrap().join("target");
    Ok(target_dir.join(profile).join("pavexc"))
}
