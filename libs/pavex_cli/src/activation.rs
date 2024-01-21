//! This is a temporary stop-gap solution to allow us to publish Pavex to crates.io
//! without giving everyone access.  
//! The "secret" product key is available in Pavex's Discord server for every person that's
//! of the beta.
//! That's why we just check a SHA2 hash of the key rather than doing something more robust
//! with `argon2` or `bcrypt`.
//!
//! In the future we'll migrate to a more robust solution, with per-user keys,
//! validation checks against a server at most once every X hours, etc.
//! But this will do for now.

use crate::state::State;
use cargo_like_utils::shell::Shell;
use secrecy::{ExposeSecret, SecretString};
use sha2::Digest;

static BETA_ACTIVATION_KEY_SHA256: &str =
    "7b027f749555e3a063b012caaa2508094eee34233f1bdc7a0b8e1f1f19627c1d";

/// Verify that Pavex has been correctly activated on this machine.
pub fn check_activation(state: &State, shell: &mut Shell) -> Result<(), anyhow::Error> {
    let key = state.get_activation_key(shell)?;
    let Some(key) = key else {
        return Err(PavexMustBeActivated.into());
    };
    check_activation_key(&key)?;
    Ok(())
}

/// Verify that the given activation key is valid.
pub fn check_activation_key(key: &SecretString) -> Result<(), InvalidActivationKey> {
    let key_sha256 = sha2::Sha256::digest(key.expose_secret().as_bytes());
    let key_sha256 = hex::encode(key_sha256);
    if key_sha256 != BETA_ACTIVATION_KEY_SHA256 {
        Err(InvalidActivationKey)
    } else {
        Ok(())
    }
}

#[derive(thiserror::Error, Debug)]
#[error("Your installation of Pavex must be activated before it can be used.\nRun `pavex self activate` to fix the issue.")]
pub struct PavexMustBeActivated;

#[derive(thiserror::Error, Debug)]
#[error("The activation key attached to your installation of Pavex is not valid.\nRun `pavex self activate` to fix the issue.")]
pub struct InvalidActivationKey;
