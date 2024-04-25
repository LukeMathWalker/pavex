use crate::activation::token::{ActivationProof, CliToken};
use crate::command::Command;
use crate::locator::PavexLocator;
use crate::state::State;
use anyhow::Context;
use cargo_like_utils::shell::Shell;
use jsonwebtoken::jwk::JwkSet;
use redact::Secret;
use token_cache::CliTokenDiskCache;

mod token;
mod token_cache;

/// If the command requires it, check if Pavex has been correctly activated
/// on this machine.
pub fn check_activation_if_necessary(
    command: &Command,
    locator: &PavexLocator,
    shell: &mut Shell,
    key_set: &JwkSet,
) -> Result<(), anyhow::Error> {
    if !command.needs_activation_key() {
        return Ok(());
    }
    check_activation(locator, shell, key_set)?;
    Ok(())
}

/// Verify that Pavex has been correctly activated on this machine.
pub fn check_activation(
    locator: &PavexLocator,
    shell: &mut Shell,
    key_set: &JwkSet,
) -> Result<ActivationProof, anyhow::Error> {
    let state = State::new(&locator);
    let key = state.get_activation_key(shell)?;
    let Some(key) = key else {
        return Err(PavexMustBeActivated.into());
    };
    check_activation_with_key(locator, key, key_set).map_err(Into::into)
}

/// Verify that the given activation key is valid.
pub fn check_activation_with_key(
    locator: &PavexLocator,
    key: Secret<String>,
    key_set: &JwkSet,
) -> Result<ActivationProof, CliTokenError> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(_check_activation_with_key(locator, key, key_set))
}

async fn _check_activation_with_key(
    locator: &PavexLocator,
    activation_key: Secret<String>,
    key_set: &JwkSet,
) -> Result<ActivationProof, CliTokenError> {
    let cache = CliTokenDiskCache::new(locator.auth());
    let cached_jwt = match CliToken::from_cache(&cache) {
        Ok(Some(jwt)) => Some(jwt),
        Ok(None) => {
            tracing::info!("No CLI token was found on disk. Obtaining a new one from Pavex's API.");
            None
        }
        Err(e) => {
            tracing::error!(error.msg = %e, error.details = ?e, "Failed to retrieve a cached token from disk", );
            None
        }
    };

    let mut validation_proof = None;
    if let Some(cached_jwt) = cached_jwt {
        match cached_jwt.validate(key_set) {
            Err(e) => {
                tracing::warn!(
                    error.msg = %e,
                    error.details = ?e,
                    "The cached CLI token is invalid. Obtaining a new one from Pavex's API.",
                );
            }
            Ok(proof) => {
                validation_proof = Some(proof);
            }
        }
    }

    let validation_proof = match validation_proof {
        None => {
            let jwt = CliToken::from_api(activation_key).await?;
            jwt.validate(key_set)
                .context("The token retrieved from Pavex's API is invalid.")
                .map_err(CliTokenError::RpcError)?
        }
        Some(validation_proof) => validation_proof,
    };
    Ok(validation_proof)
}

#[derive(thiserror::Error, Debug)]
#[error("Your installation of Pavex must be activated before it can be used.\nRun `pavex self activate` to fix the issue.")]
pub struct PavexMustBeActivated;

#[derive(thiserror::Error, Debug)]
#[error("The activation key attached to your installation of Pavex is not valid.\nRun `pavex self activate` to fix the issue.")]
pub struct InvalidActivationKey;

#[derive(thiserror::Error, Debug)]
pub enum CliTokenError {
    #[error(transparent)]
    ActivationKey(#[from] InvalidActivationKey),
    #[error("Failed to exchange your activation key for a CLI token with api.pavex.dev")]
    RpcError(#[source] anyhow::Error),
}
