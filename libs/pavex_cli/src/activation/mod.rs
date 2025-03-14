use crate::activation::token::ValidatedClaims;
use crate::command::Command;
use crate::locator::PavexLocator;
use crate::state::State;
use anyhow::Context;
use jiff::{SignedDuration, Timestamp};
use jsonwebtoken::jwk::JwkSet;
use pavex_cli_diagnostic::AnyhowBridge;
use redact::Secret;
use token_cache::CliTokenDiskCache;
use tracing_log_error::log_error;

pub use http_client::HTTP_CLIENT;
pub use token::CliToken;
pub use wizard_key::WizardKey;

mod http_client;
mod json_api;
mod token;
mod token_cache;
mod wizard_key;

/// If the command requires it, retrieve Pavex's activation key from the state.
pub fn get_activation_key_if_necessary(
    command: &Command,
    locator: &PavexLocator,
) -> Result<Option<Secret<String>>, anyhow::Error> {
    if !command.needs_activation_key() {
        return Ok(None);
    }
    get_activation_key(locator).map(Some)
}

/// Retrieve Pavex's activation key from the state.
pub fn get_activation_key(locator: &PavexLocator) -> Result<Secret<String>, anyhow::Error> {
    let state = State::new(locator);
    let key = state.get_activation_key()?;
    let Some(key) = key else {
        return Err(PavexMustBeActivated.into());
    };
    Ok(key)
}

/// Exchange a wizard key for a CLI token and an activation key.
///
/// Then store both on disk for future use.
pub fn exchange_wizard_key(
    locator: &PavexLocator,
    wizard_key: Secret<String>,
) -> Result<(), miette::Error> {
    let wizard_key = WizardKey::new(wizard_key);
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async move {
        let (activation_key, cli_token) = wizard_key.exchange().await?;
        let state = State::new(locator);
        state
            .set_activation_key(activation_key)
            .map_err(|e| e.into_miette())?;
        let cache = CliTokenDiskCache::new(locator.auth());
        cache
            .upsert_token(cli_token.into())
            .await
            .map_err(|e| e.into_miette())?;
        Ok(())
    })
}

pub fn background_token_refresh(
    latest_claims: &ValidatedClaims,
    key_set: &JwkSet,
    activation_key: Secret<String>,
    locator: &PavexLocator,
) {
    if *latest_claims.issued_at() + SignedDuration::from_mins(10) > Timestamp::now() {
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
                log_error!(*e, level: tracing::Level::WARN, "Failed to refresh the CLI token in the background");
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
        .upsert_token(jwt.raw().clone())
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
            log_error!(*e, "Failed to retrieve a cached token from disk");
            None
        }
    };

    let mut claims = None;
    if let Some(cached_jwt) = cached_jwt {
        match cached_jwt.validate(key_set) {
            Err(e) => {
                log_error!(*e, level: tracing::Level::WARN, "The cached CLI token is invalid. Obtaining a new one from Pavex's API.");
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
            if let Err(e) = cache.upsert_token(jwt.raw().clone()).await {
                log_error!(*e, level: tracing::Level::WARN, "Failed to save the fresh CLI token to disk");
            }

            claims
        }
        Some(claims) => {
            tracing::info!(
                "The cached CLI token is still valid. Using it rather than fetching a new one."
            );
            claims
        }
    };
    Ok(claims)
}

#[derive(thiserror::Error, Debug)]
#[error(
    "Your installation of Pavex must be activated before it can be used.\nRun `pavex self activate` to fix the issue."
)]
pub struct PavexMustBeActivated;

#[derive(thiserror::Error, Debug)]
#[error(
    "The activation key attached to your installation of Pavex is not valid.\nRun `pavex self activate` to fix the issue."
)]
pub struct InvalidActivationKey;

#[derive(thiserror::Error, Debug)]
pub enum CliTokenError {
    #[error(transparent)]
    ActivationKey(#[from] InvalidActivationKey),
    #[error("Failed to exchange your activation key for a CLI token with api.pavex.dev")]
    RpcError(#[source] anyhow::Error),
}

#[derive(thiserror::Error, miette::Diagnostic, Debug)]
#[error(
    "The wizard key you provided is malformed. {details}",
    details = .details.as_deref().unwrap_or_default()
)]
#[diagnostic(help(
    "Try copying the key again from the browser (https://console.pavex.dev/wizard/setup). Perhaps you lost a piece along the way?"
))]
pub struct MalformedWizardKey {
    details: Option<String>,
}

#[derive(thiserror::Error, miette::Diagnostic, Debug)]
#[error("The wizard key you provided is either invalid or expired.")]
#[diagnostic(help(
    "Refresh the setup page (https://console.pavex.dev/wizard/setup) in the browser to generate a new key."
))]
pub struct InvalidWizardKey;

#[derive(thiserror::Error, miette::Diagnostic, Debug)]
pub enum WizardKeyError {
    #[error(transparent)]
    #[diagnostic(transparent)]
    MalformedKey(#[from] MalformedWizardKey),
    #[error(transparent)]
    #[diagnostic(transparent)]
    InvalidKey(#[from] InvalidWizardKey),
    #[error(
        "Something went wrong when trying to exchange your wizard key for an activation token."
    )]
    #[diagnostic(help(
        "Please try again! If the problem persists, send us a message in the #get-help channel on Discord."
    ))]
    UnexpectedError(#[source] anyhow::Error),
}
