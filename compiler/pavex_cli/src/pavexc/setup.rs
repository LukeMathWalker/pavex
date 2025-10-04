use std::{path::Path, process::Stdio};

use anyhow::Context;

/// Ensure that the required dependencies are installed for this version of `pavexc`.
pub(super) fn pavexc_setup(cli_path: &Path) -> Result<(), anyhow::Error> {
    let mut cmd = std::process::Command::new(cli_path);
    cmd.arg("self").arg("setup");
    let cmd_debug = format!("{:?}", &cmd);
    let output = cmd
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
        .with_context(|| format!("`{cmd_debug}` failed"))?;
    if !output.status.success() {
        anyhow::bail!("`{cmd_debug}` failed");
    }
    Ok(())
}
