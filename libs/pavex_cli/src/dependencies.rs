use anyhow::Context;
use std::process::Stdio;

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

/// Install the nightly toolchain via `rustup`.
pub fn install_nightly() -> Result<(), anyhow::Error> {
    let mut cmd = std::process::Command::new("rustup");
    cmd.arg("toolchain").arg("install").arg("nightly");
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

/// Check if the nightly toolchain is installed via `rustup`.
pub fn is_rustdoc_json_installed() -> Result<(), anyhow::Error> {
    let mut cmd = std::process::Command::new("rustup");
    cmd.arg("component")
        .arg("list")
        .arg("--installed")
        .arg("--toolchain")
        .arg("nightly");
    let cmd_debug = format!("{:?}", &cmd);
    let output = cmd
        .output()
        .with_context(|| format!("`{cmd_debug}` failed"))?;
    if !output.status.success() {
        anyhow::bail!("`{cmd_debug}` failed");
    }
    let stdout = std::str::from_utf8(&output.stdout)
        .with_context(|| format!("`{cmd_debug}` returned invalid UTF8"))?;
    if stdout
        .lines()
        .any(|l| l.trim().starts_with("rust-docs-json"))
    {
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "`rust-docs-json` component is not installed for the nightly toolchain"
        ))
    }
}

/// Install the `rust-docs-json` component for the nightly toolchain via `rustup`.
pub fn install_rustdoc_json() -> Result<(), anyhow::Error> {
    let mut cmd = std::process::Command::new("rustup");
    cmd.arg("component")
        .arg("add")
        .arg("rust-docs-json")
        .arg("--toolchain")
        .arg("nightly");
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

/// Check if `cargo px` is installed.
pub fn is_cargo_px_installed() -> Result<(), anyhow::Error> {
    let mut cmd = std::process::Command::new("cargo");
    cmd.arg("px").arg("-V");
    let cmd_debug = format!("{:?}", &cmd);
    let output = cmd
        .output()
        .with_context(|| format!("`{cmd_debug}` failed"))?;
    if !output.status.success() {
        anyhow::bail!("`{cmd_debug}` failed");
    }
    Ok(())
}
