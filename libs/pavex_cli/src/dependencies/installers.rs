use crate::command::Color;
use crate::dependencies::{
    install_nightly, install_rustdoc_json, is_cargo_px_installed, is_nightly_installed,
    is_rustdoc_json_installed, is_rustup_installed,
};
use crate::locator::PavexLocator;
use crate::user_input::confirm;
use anyhow::Error;
use cargo_like_utils::shell::{style, Shell};
use std::io::IsTerminal;

pub trait Dependency {
    const NAME: &'static str;
    const INSTALLATION_INSTRUCTIONS: &'static str;
    const AUTO_INSTALLABLE: bool = false;

    fn auto_install() -> Result<(), anyhow::Error> {
        if Self::AUTO_INSTALLABLE {
            unimplemented!()
        } else {
            unreachable!()
        }
    }

    fn is_installed() -> Result<(), Error>;
}

pub fn verify_installation<D: Dependency>(shell: &mut Shell) -> Result<(), anyhow::Error> {
    let _ = shell.status("Checking", format!("if {} is installed", D::NAME));
    if let Err(mut e) = is_nightly_installed() {
        let _ = shell.status_with_color(
            "Missing",
            format!("{} is not installed\n", D::NAME),
            &style::ERROR,
        );
        let mut installed = false;
        if std::io::stdout().is_terminal() && D::AUTO_INSTALLABLE {
            let prompt = format!("\tShould I install {} for you?", D::NAME);
            if let Ok(true) = confirm(&prompt, true) {
                if let Err(inner) = D::auto_install() {
                    e = inner;
                    let _ = shell.status_with_color(
                        "Failed",
                        format!("to install {}", D::NAME),
                        &style::ERROR,
                    );
                } else {
                    installed = true;
                }
            }
        }
        if !installed {
            let _ = shell.note(D::INSTALLATION_INSTRUCTIONS);
            return Err(e);
        }
    }

    let _ = shell.status("Success", format!("{} is installed", D::NAME()));
    Ok(())
}

pub struct RustdocJson;

impl Dependency for RustdocJson {
    const NAME: &'static str = "the `rust-docs-json` component";
    const INSTALLATION_INSTRUCTIONS: &'static str = r#"Invoke

    rustup component add rust-docs-json --toolchain nightly

to add the missing component and fix the issue."#;

    const AUTO_INSTALLABLE: bool = true;

    fn auto_install() -> Result<(), Error> {
        install_rustdoc_json()
    }

    fn is_installed() -> Result<(), Error> {
        is_rustdoc_json_installed()
    }
}

pub struct NightlyToolchain;

impl Dependency for NightlyToolchain {
    const NAME: &'static str = "Rust's nightly toolchain";
    const INSTALLATION_INSTRUCTIONS: &'static str = r#"Invoke

    rustup toolchain install nightly

to add the missing toolchain and fix the issue."#;
    const AUTO_INSTALLABLE: bool = true;

    fn auto_install() -> Result<(), Error> {
        install_nightly()
    }

    fn is_installed() -> Result<(), Error> {
        install_nightly()
    }
}

pub struct Rustup;

impl Dependency for Rustup {
    const NAME: &'static str = "`rustup`";
    const INSTALLATION_INSTRUCTIONS: &'static str = "Install `rustup` \
    following the instructions at https://rust-lang.org/tools/install \
    to fix the issue";
    const AUTO_INSTALLABLE: bool = false;

    fn is_installed() -> Result<(), Error> {
        is_rustup_installed()
    }
}

pub struct CargoPx;

impl Dependency for CargoPx {
    const NAME: &'static str = "`cargo-px`";
    const INSTALLATION_INSTRUCTIONS: &'static str = "Follow the instructions \
    at https://lukemathwalker.github.io/cargo-px/ to install the missing sub-command \
    and fix the issue.";

    fn is_installed() -> Result<(), Error> {
        is_cargo_px_installed()
    }
}
