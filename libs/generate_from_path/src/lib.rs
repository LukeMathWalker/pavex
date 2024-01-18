use crate::template::{create_liquid_object, LiquidObjectResource};
use crate::template_variables::{
    set_project_name_variables, CrateName, ProjectDir, ProjectName, ProjectNameInput,
};
use anyhow::bail;
use liquid::ParserBuilder;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use tracing::info;

mod filenames;
mod ignore_me;
mod progressbar;
mod template;
mod template_variables;

#[derive(Debug)]
pub struct GenerateArgs {
    pub name: String,
    pub template_dir: PathBuf,
    pub destination: PathBuf,
    pub define: HashMap<String, String>,
    pub ignore: Option<Vec<String>>,
    pub overwrite: bool,
    pub verbose: bool,
}

pub fn generate(args: GenerateArgs) -> Result<PathBuf, anyhow::Error> {
    let template_dir = get_source_template_into_temp(&args.template_dir)?;
    let project_dir = expand_template(&template_dir.path(), &args)?;

    copy_expanded_template(template_dir.path(), &project_dir, &args)
}

fn expand_template(template_dir: &Path, args: &GenerateArgs) -> anyhow::Result<PathBuf> {
    let mut liquid_object = create_liquid_object(args)?;

    let project_name_input = ProjectNameInput::from(&liquid_object);
    let destination = ProjectDir::try_from(args)?;
    let project_name = ProjectName::from(&project_name_input);
    let crate_name = CrateName::from(&project_name_input);
    set_project_name_variables(&mut liquid_object, &destination, &project_name, &crate_name)?;

    info!("Destination: {destination}");
    info!("project-name: {project_name}");
    info!("Generating template");

    add_defined_values(&mut liquid_object, &args);

    ignore_me::remove_unneeded_files(template_dir, &args.ignore, args.verbose)?;
    let mut pbar = progressbar::new();
    let liquid_engine = ParserBuilder::with_stdlib().build()?;

    template::walk_dir(
        template_dir,
        &mut liquid_object,
        &liquid_engine,
        &mut pbar,
        args.verbose,
    )?;
    Ok(destination.as_ref().to_owned())
}

fn copy_expanded_template(
    template_dir: &Path,
    project_dir: &Path,
    args: &GenerateArgs,
) -> anyhow::Result<PathBuf> {
    info!("Moving generated files into: {}", project_dir.display());
    copy_dir_all(template_dir, project_dir, args.overwrite)?;
    info!("Initializing a fresh Git repository");
    git_init(project_dir)?;
    info!("Done! New project created in {}", project_dir.display());
    Ok(project_dir.to_owned())
}

/// Use the `git` command line tool to initialize a new repository
/// at the given `project_dir`.
fn git_init(project_dir: &Path) -> anyhow::Result<()> {
    let output = std::process::Command::new("git")
        .arg("init")
        .arg("-b")
        .arg("main")
        .arg(project_dir)
        .output()?;
    if !output.status.success() {
        bail!(
            "Failed to initialize git repository at {}: {}",
            project_dir.display(),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

fn add_defined_values(liquid_object: &mut LiquidObjectResource, generate_args: &GenerateArgs) {
    for (key, value) in &generate_args.define {
        liquid_object.insert(
            key.into(),
            liquid_core::Value::Scalar(value.to_owned().into()),
        );
    }
}

fn get_source_template_into_temp(template_dir: &Path) -> anyhow::Result<TempDir> {
    let temp_dir = tempfile::Builder::new().prefix("pavex-new").tempdir()?;
    copy_dir_all(template_dir, temp_dir.path(), false)?;
    Ok(temp_dir)
}

pub(crate) fn copy_dir_all(
    src: impl AsRef<Path>,
    dst: impl AsRef<Path>,
    overwrite: bool,
) -> anyhow::Result<()> {
    fn check_dir_all(
        src: impl AsRef<Path>,
        dst: impl AsRef<Path>,
        overwrite: bool,
    ) -> anyhow::Result<()> {
        if !dst.as_ref().exists() {
            return Ok(());
        }

        for src_entry in fs_err::read_dir(src.as_ref())? {
            let src_entry = src_entry?;
            let filename = src_entry.file_name().to_string_lossy().to_string();
            let entry_type = src_entry.file_type()?;

            if entry_type.is_dir() {
                if filename == ".git" {
                    continue;
                }
                let dst_path = dst.as_ref().join(filename);
                check_dir_all(src_entry.path(), dst_path, overwrite)?;
            } else if entry_type.is_file() {
                let filename = filename.strip_suffix(".liquid").unwrap_or(&filename);
                let dst_path = dst.as_ref().join(filename);
                match (dst_path.exists(), overwrite) {
                    (true, false) => {
                        bail!("File already exists: {}", dst_path.display())
                    }
                    (true, true) => {
                        tracing::warn!("Overwriting file: {}", dst_path.display());
                    }
                    _ => {}
                };
            } else {
                bail!("Symbolic links not supported")
            }
        }
        Ok(())
    }
    fn copy_all(
        src: impl AsRef<Path>,
        dst: impl AsRef<Path>,
        overwrite: bool,
    ) -> anyhow::Result<()> {
        fs_err::create_dir_all(&dst)?;
        for src_entry in fs_err::read_dir(src.as_ref())? {
            let src_entry = src_entry?;
            let filename = src_entry.file_name().to_string_lossy().to_string();
            let entry_type = src_entry.file_type()?;
            if entry_type.is_dir() {
                let dst_path = dst.as_ref().join(filename);
                if ".git" == src_entry.file_name() {
                    continue;
                }
                copy_dir_all(src_entry.path(), dst_path, overwrite)?;
            } else if entry_type.is_file() {
                let filename = filename.strip_suffix(".liquid").unwrap_or(&filename);
                let dst_path = dst.as_ref().join(filename);
                if dst_path.exists() && overwrite {
                    fs_err::remove_file(&dst_path)?;
                }
                fs_err::copy(src_entry.path(), dst_path)?;
            }
        }
        Ok(())
    }

    check_dir_all(&src, &dst, overwrite)?;
    copy_all(src, dst, overwrite)
}
