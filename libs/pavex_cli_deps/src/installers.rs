use crate::commands::{
    install_rustdoc_json, install_rustup_toolchain, is_cargo_px_installed,
    is_rustdoc_json_installed, is_rustup_installed, is_rustup_toolchain_installed,
};
use crate::user_input::confirm;
use anyhow::Error;
use cargo_like_utils::shell::style;
use pavex_cli_shell::{SHELL, ShellExt};
use std::borrow::Cow;
use std::io::IsTerminal;

/// An interface for system dependencies that may or may not be auto-installable.
pub trait Dependency {
    /// The dependency name, primarily used in error messages.
    fn name(&self) -> Cow<'_, str>;

    /// A user-facing message explaining how to install this dependency
    /// if it's missing.
    fn installation_instructions(&self) -> Cow<'_, str>;

    /// Returns `true` if the dependency can be installed without
    /// the user's intervention.
    fn is_auto_installable(&self) -> bool;

    fn auto_install(&self) -> Result<(), anyhow::Error> {
        if self.is_auto_installable() {
            unimplemented!()
        } else {
            unreachable!()
        }
    }

    fn is_installed(&self) -> Result<(), Error>;
}

/// What to do if a dependency is missing and it can be autoinstalled.
#[derive(Clone, Copy)]
pub enum IfAutoinstallable {
    PromptForConfirmation,
    Autoinstall,
    PrintInstructions,
}

/// Verify that a certain dependency is installed.
/// It returns an error if it isn't.
pub fn verify_installation<D: Dependency>(
    dep: D,
    if_autoinstallable: IfAutoinstallable,
) -> Result<(), VerifyInstallationError> {
    let name = dep.name();
    SHELL.status("Checking", format!("if {name} is installed"));
    if let Err(mut e) = dep.is_installed() {
        SHELL.status_with_color(
            "Missing",
            format!("{name} is not installed\n"),
            &style::ERROR,
        );
        let mut installed = false;
        let mut attempted_auto_install = false;
        if dep.is_auto_installable() {
            let auto_install = match if_autoinstallable {
                IfAutoinstallable::PromptForConfirmation => {
                    if std::io::stdout().is_terminal() {
                        let prompt = format!("\tShould I install {name} for you?");
                        matches!(confirm(&prompt, true), Ok(true))
                    } else {
                        false
                    }
                }
                IfAutoinstallable::Autoinstall => {
                    SHELL.status("Installing", format!("{name}"));
                    true
                }
                IfAutoinstallable::PrintInstructions => false,
            };
            if auto_install {
                match dep.auto_install() {
                    Err(inner) => {
                        attempted_auto_install = true;
                        e = inner;
                    }
                    _ => {
                        installed = true;
                    }
                }
            }
        }
        if !installed {
            return Err(VerifyInstallationError {
                name: name.into_owned(),
                attempted_auto_install,
                instructions: dep.installation_instructions().into_owned(),
                source: e,
            });
        }
    }

    SHELL.status("Success", format!("{name} is installed"));
    Ok(())
}

#[derive(Debug, miette::Diagnostic)]
pub struct VerifyInstallationError {
    name: String,
    attempted_auto_install: bool,
    #[help]
    pub instructions: String,
    source: Error,
}

impl std::fmt::Display for VerifyInstallationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "`{} must be installed for Pavex to work as expected.",
            &self.name
        )?;
        if self.attempted_auto_install {
            write!(
                f,
                " I tried to install it for you, but I didn't succeed. \
                Try installing it manually by following the instructions below."
            )?;
        }
        Ok(())
    }
}

impl std::error::Error for VerifyInstallationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.source.as_ref())
    }
}

/// The `rust-docs-json` component of a Rust toolchain.
pub struct RustdocJson {
    /// The name of the Rust toolchain to be checked.
    pub toolchain: String,
}

impl Dependency for RustdocJson {
    fn name(&self) -> Cow<'_, str> {
        format!("the `rust-docs-json` component for `{}`", self.toolchain).into()
    }

    fn installation_instructions(&self) -> Cow<'_, str> {
        format!(
            "Invoke

    rustup component add rust-docs-json --toolchain {}

to add the missing component and fix the issue.",
            self.toolchain
        )
        .into()
    }

    fn is_auto_installable(&self) -> bool {
        true
    }

    fn auto_install(&self) -> Result<(), Error> {
        install_rustdoc_json(&self.toolchain)
    }

    fn is_installed(&self) -> Result<(), Error> {
        is_rustdoc_json_installed(&self.toolchain)
    }
}

/// A Rust toolchain managed by `rustup`.
pub struct RustupToolchain {
    /// The toolchain name. It must be valid when used as target in `rustup toolchain install`.
    pub name: String,
}

impl Dependency for RustupToolchain {
    fn name(&self) -> Cow<'_, str> {
        format!("the `{}` Rust toolchain", self.name).into()
    }

    fn installation_instructions(&self) -> Cow<'_, str> {
        format!(
            "Invoke

    rustup toolchain install {} -c rust-docs-json

to add the missing toolchain and fix the issue.",
            &self.name
        )
        .into()
    }

    fn is_auto_installable(&self) -> bool {
        // If `rustup` is installed.
        true
    }

    fn auto_install(&self) -> Result<(), Error> {
        install_rustup_toolchain(self)
    }

    fn is_installed(&self) -> Result<(), Error> {
        is_rustup_toolchain_installed(self)
    }
}

/// `rustup`, Rust's official toolchain manager.
pub struct Rustup;

impl Dependency for Rustup {
    fn is_installed(&self) -> Result<(), Error> {
        is_rustup_installed()
    }

    fn name(&self) -> Cow<'_, str> {
        "`rustup`".into()
    }

    fn installation_instructions(&self) -> Cow<'_, str> {
        "Install `rustup` \
        following the instructions at https://rust-lang.org/tools/install \
        to fix the issue"
            .into()
    }

    fn is_auto_installable(&self) -> bool {
        false
    }
}

/// `cargo-px`, the `cargo` sub-command required to build Pavex projects.
pub struct CargoPx;

impl Dependency for CargoPx {
    fn is_installed(&self) -> Result<(), Error> {
        is_cargo_px_installed()
    }

    fn name(&self) -> Cow<'_, str> {
        "`cargo-px`".into()
    }

    fn installation_instructions(&self) -> Cow<'_, str> {
        "Follow the instructions \
            at https://lukemathwalker.github.io/cargo-px/ to install the missing sub-command \
            and fix the issue."
            .into()
    }

    fn is_auto_installable(&self) -> bool {
        false
    }
}
