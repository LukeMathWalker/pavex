use anyhow::Context;

/// Check if `rustup` is installed and available in the system's $PATH.
pub fn is_rustup_installed() -> Result<(), anyhow::Error> {
    let mut cmd = std::process::Command::new("rustup");
    cmd.arg("--version");
    let cmd_debug = format!("{:?}", &cmd);
    let output = cmd
        .output()
        .with_context(|| format!("`{cmd_debug}` failed"))?;
    if !output.status.success() {
        anyhow::bail!("`{cmd_debug}` failed");
    }
    Ok(())
}

/// Check if the nightly toolchain is installed via `rustup`.
pub fn is_nightly_installed() -> Result<(), anyhow::Error> {
    let mut cmd = std::process::Command::new("rustup");
    cmd.arg("which")
        .arg("--toolchain")
        .arg("nightly")
        .arg("cargo");
    let cmd_debug = format!("{:?}", &cmd);
    let output = cmd
        .output()
        .with_context(|| format!("`{cmd_debug}` failed"))?;
    if !output.status.success() {
        anyhow::bail!("`{cmd_debug}` failed");
    }
    Ok(())
}
