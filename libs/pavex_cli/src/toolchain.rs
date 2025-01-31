use crate::locator::PavexLocator;
use crate::pavexc::get_or_install_from_version;
use crate::state::State;
use std::path::PathBuf;

/// Retrieve the path to the default `pavexc` binary, according to the value of
/// the default toolchain.
///
/// This is primarily used when executing `pavex` commands that don't operate within the
/// context of a Pavex project—e.g. `pavex new`.
/// Otherwise the toolchain is determined by the project's `pavex` library crate version.
pub fn get_default_pavexc(locator: &PavexLocator, state: &State) -> Result<PathBuf, anyhow::Error> {
    let version = state.get_current_toolchain()?;
    let pavexc_cli_path = get_or_install_from_version(locator, &version)?;
    Ok(pavexc_cli_path)
}
