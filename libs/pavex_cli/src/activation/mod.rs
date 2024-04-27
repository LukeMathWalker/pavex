use crate::activation::token::{CliToken, ValidatedClaims};
use crate::command::Command;
use crate::locator::PavexLocator;
use crate::state::State;
use anyhow::Context;
use cargo_like_utils::shell::Shell;
use jsonwebtoken::jwk::JwkSet;
use redact::Secret;
use time::Duration;
use token_cache::CliTokenDiskCache;

mod token;
mod token_cache;

/// If the command requires it, retrieve Pavex's activation key from the state.
pub fn get_activation_key_if_necessary(
    command: &Command,
    locator: &PavexLocator,
    shell: &mut Shell,
) -> Result<Option<Secret<String>>, anyhow::Error> {
    if !command.needs_activation_key() {
        return Ok(None);
    }
    get_activation_key(locator, shell).map(Some)
}

/// Retrieve Pavex's activation key from the state.
pub fn get_activation_key(
    locator: &PavexLocator,
    shell: &mut Shell,
) -> Result<Secret<String>, anyhow::Error> {
    let state = State::new(&locator);
    let key = state.get_activation_key(shell)?;
    let Some(key) = key else {
        return Err(PavexMustBeActivated.into());
    };
    Ok(key)
}

pub fn background_token_refresh(
    latest_claims: &ValidatedClaims,
    key_set: &JwkSet,
    activation_key: Secret<String>,
    locator: &PavexLocator,
) {
    if time::OffsetDateTime::now_utc() - latest_claims.issued_at().to_owned()
        < Duration::minutes(10)
    {
        // The token is super fresh, no need to refresh it.
        return;
    }
    let locator = locator.to_owned();
    let key_set = key_set.to_owned();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            if let Err(e) = _token_refresh(&key_set, &locator, activation_key).await {
                tracing::warn!(error.msg = %e, error.details = ?e, "Failed to refresh the CLI token in the background")
            }
        });
    });
}

async fn _token_refresh(
    key_set: &JwkSet,
    locator: &PavexLocator,
    activation_key: Secret<String>,
) -> Result<ValidatedClaims, anyhow::Error> {
    let jwt = CliToken::from_api(activation_key).await?;
    let claims = jwt
        .validate(key_set)
        .context("The token retrieved from Pavex's API is invalid.")
        .map_err(CliTokenError::RpcError)?;
    CliTokenDiskCache::new(locator.auth())
        .update_token(jwt.raw().clone())
        .await?;
    Ok(claims)
}

/// Verify that Pavex has been correctly activated on this machine.
pub fn check_activation(
    locator: &PavexLocator,
    key: Secret<String>,
    key_set: &JwkSet,
) -> Result<ValidatedClaims, CliTokenError> {
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
) -> Result<ValidatedClaims, CliTokenError> {
    let cache = CliTokenDiskCache::new(locator.auth());
    let cached_jwt = match CliToken::from_cache(&cache).await {
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

    let mut claims = None;
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
                claims = Some(proof);
            }
        }
    }

    let claims = match claims {
        None => {
            let jwt = CliToken::from_api(activation_key).await?;
            let claims = jwt
                .validate(key_set)
                .context("The token retrieved from Pavex's API is invalid.")
                .map_err(CliTokenError::RpcError)?;

            // We have a fresh token. Let's cache it to disk to avoid hitting the API
            // the next time Pavex CLI is invoked.
            if let Err(e) = cache.update_token(jwt.raw().clone()).await {
                tracing::warn!(
                    error.msg = %e,
                    error.details = ?e,
                    "Failed to save the fresh CLI token to disk",
                );
            }

            claims
        }
        Some(claims) => claims,
    };
    Ok(claims)
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
