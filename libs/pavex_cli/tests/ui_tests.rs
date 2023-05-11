use pavex_test_runner::run_tests;
use std::path::PathBuf;
use std::str::FromStr;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = PathBuf::from_str(env!("CARGO_MANIFEST_DIR")).unwrap();
    let test_data_folder = manifest_dir.join("tests").join("ui_tests");
    let test_runtime_folder = manifest_dir.parent().unwrap().join("ui_test_envs");
    run_tests(test_data_folder, test_runtime_folder)?.exit();
}
