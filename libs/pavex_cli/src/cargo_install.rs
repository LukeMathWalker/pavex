use std::path::Path;
use std::process::Stdio;

pub enum Source {
    CratesIo { name: String, version: String },
    Git { url: String, rev: GitSourceRevision },
    Local { path: String },
}

pub enum GitSourceRevision {
    Branch(String),
    Tag(String),
    Rev(String),
}

/// Install a single binary via `cargo install` and copy it to the specified destination path.
pub fn cargo_install(
    source: Source,
    binary_name: &str,
    destination: &Path,
) -> Result<(), anyhow::Error> {
    let temp_dir = tempfile::tempdir()?;
    let mut cmd = std::process::Command::new("cargo");
    cmd.arg("install").arg("--root").arg(temp_dir.path());
    match source {
        Source::CratesIo { name, version } => {
            cmd.arg("--version");
            cmd.arg(&version);
            cmd.arg(&name);
        }
        Source::Git { url, rev } => {
            cmd.arg("--git");
            cmd.arg(&url);
            match rev {
                GitSourceRevision::Branch(branch) => {
                    cmd.arg("--branch");
                    cmd.arg(&branch);
                }
                GitSourceRevision::Tag(tag) => {
                    cmd.arg("--tag");
                    cmd.arg(&tag);
                }
                GitSourceRevision::Rev(rev) => {
                    cmd.arg("--rev");
                    cmd.arg(&rev);
                }
            }
        }
        Source::Local { path } => {
            cmd.arg("--path");
            cmd.arg(&path);
        }
    }
    let output = cmd
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()?;
    if !output.status.success() {
        anyhow::bail!("`cargo install` failed");
    }
    fs_err::copy(temp_dir.path().join("bin").join(binary_name), destination)?;
    Ok(())
}
