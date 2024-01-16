use anyhow::Context;
use std::path::Path;
use std::process::Stdio;

pub enum Source {
    CratesIo { version: String },
    Git { url: String, rev: GitSourceRevision },
}

pub enum GitSourceRevision {
    Rev(String),
    Tag(String),
    Branch(String),
}

/// Install a single binary via `cargo install` and copy it to the specified destination path.
pub fn cargo_install(
    source: Source,
    binary_name: &str,
    package_name: &str,
    destination: &Path,
) -> Result<(), anyhow::Error> {
    let temp_dir = tempfile::tempdir()?;
    let mut cmd = std::process::Command::new("cargo");
    cmd.arg("install")
        .arg("--root")
        .arg(temp_dir.path())
        .arg("--bin")
        .arg(binary_name);
    match source {
        Source::CratesIo { version } => {
            cmd.arg("--version");
            cmd.arg(&version);
        }
        Source::Git { url, rev } => {
            cmd.arg("--git");
            cmd.arg(&url);
            match rev {
                GitSourceRevision::Rev(rev) => {
                    cmd.arg("--rev");
                    cmd.arg(&rev);
                }
                GitSourceRevision::Tag(tag) => {
                    cmd.arg("--tag");
                    cmd.arg(&tag);
                }
                GitSourceRevision::Branch(branch) => {
                    cmd.arg("--branch");
                    cmd.arg(&branch);
                }
            }
        }
    }
    cmd.arg(&package_name);
    let cmd_debug = format!("{:?}", &cmd);
    let output = cmd
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
        .with_context(|| format!("`{cmd_debug}` failed"))?;
    if !output.status.success() {
        anyhow::bail!("`{cmd_debug}` failed");
    }
    fs_err::copy(temp_dir.path().join("bin").join(binary_name), destination)?;
    Ok(())
}
