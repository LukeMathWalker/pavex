mod crate_name;
mod project_dir;
mod project_name;
mod project_name_input;

use crate::template::LiquidObjectResource;
pub use crate_name::CrateName;
use liquid_core::Value;
pub use project_dir::ProjectDir;
pub use project_name::ProjectName;
pub use project_name_input::ProjectNameInput;
use std::path::Path;

pub fn set_project_name_variables(
    liquid_object: &mut LiquidObjectResource,
    project_dir: &ProjectDir,
    project_name: &ProjectName,
    crate_name: &CrateName,
) -> anyhow::Result<()> {
    liquid_object.insert(
        "project-name".into(),
        Value::Scalar(project_name.as_ref().to_owned().into()),
    );

    liquid_object.insert(
        "crate_name".into(),
        Value::Scalar(crate_name.as_ref().to_owned().into()),
    );

    liquid_object.insert(
        "within_cargo_project".into(),
        Value::Scalar(is_within_cargo_project(project_dir.as_ref()).into()),
    );

    Ok(())
}

fn is_within_cargo_project(project_dir: &Path) -> bool {
    Path::new(project_dir)
        .ancestors()
        .any(|folder| folder.join("Cargo.toml").exists())
}
