use crate::activation::token_cache::CliTokenDiskCache;
use crate::activation::{CliTokenError, InvalidActivationKey};
use anyhow::Context;
use jsonwebtoken::jwk::{JwkSet, KeyAlgorithm};
use jsonwebtoken::{decode_header, Algorithm, DecodingKey, TokenData};
use redact::Secret;
use reqwest_retry::policies::ExponentialBackoff;
use reqwest_retry::RetryTransientMiddleware;
use reqwest_tracing::TracingMiddleware;
use std::collections::HashSet;

/// A token obtained from Pavex's API using a valid activation key.
///
/// `CliToken` doesn't guarantee that the token is valid!
/// Use [`CliToken::validate`] to obtain a validation proof.
pub struct CliToken(Secret<String>);

impl CliToken {
    /// Retrieve a CLI token from the disk cache if it contains one.
    pub async fn from_cache(cache: &CliTokenDiskCache) -> Result<Option<Self>, anyhow::Error> {
        cache.get_token().await.map(|t| t.map(CliToken))
    }

    /// Get a fresh CLI token from Pavex's API.
    pub async fn from_api(activation_key: Secret<String>) -> Result<Self, CliTokenError> {
        #[derive(serde::Serialize)]
        struct Request {
            #[serde(serialize_with = "redact::expose_secret")]
            activation_key: Secret<String>,
        }

        #[derive(serde::Deserialize)]
        struct Response {
            jwt: Secret<String>,
        }

        let retry_policy = ExponentialBackoff::builder().build_with_max_retries(3);
        let client = reqwest_middleware::ClientBuilder::new(reqwest::Client::new())
            .with(TracingMiddleware::default())
            .with(RetryTransientMiddleware::new_with_policy(retry_policy))
            .build();

        let user_agent = format!("pavex-cli/{}", env!("CARGO_PKG_VERSION"));
        let response = client
            .post("https://api.pavex.dev/v1/cli/login")
            .header("User-Agent", user_agent)
            .json(&Request { activation_key })
            .send()
            .await
            .map_err(|e| CliTokenError::RpcError(e.into()))?;
        match response.error_for_status() {
            Ok(response) => {
                let response: Response = response
                    .json()
                    .await
                    .map_err(|e| CliTokenError::RpcError(e.into()))?;
                Ok(Self(response.jwt))
            }
            Err(e) => {
                if let Some(status) = e.status() {
                    if status == reqwest::StatusCode::FORBIDDEN {
                        return Err(InvalidActivationKey.into());
                    }
                }
                Err(CliTokenError::RpcError(e.into()))
            }
        }
    }

    pub fn validate(&self, jwks: &JwkSet) -> Result<ValidatedClaims, anyhow::Error> {
        let header = decode_header(&self.0.expose_secret())
            .context("Failed to decode the JOSE header of the CLI token")?;
        let kid = header.kid.ok_or_else(|| {
            anyhow::anyhow!("The CLI token is missing the key id (`kid`) in its JOSE header")
        })?;
        let jwk = jwks.find(&kid).ok_or_else(|| {
            anyhow::anyhow!("The CLI token references a key id (`kid`) that is not in the JWKS")
        })?;
        let key_algorithm = jwk.common.key_algorithm.ok_or_else(|| {
            anyhow::anyhow!("The JWK referenced by the CLI token is missing the key algorithm")
        })?;
        let decoding_key =
            DecodingKey::from_jwk(jwk).context("Failed to create a decoding key from the JWK")?;

        let mut validation = jsonwebtoken::Validation::new(key_algo2algo(key_algorithm)?);
        validation.aud = Some(HashSet::from_iter(["pavex_cli".to_string()]));
        validation.iss = Some(HashSet::from_iter(["https://api.pavex.dev".to_string()]));

        let token: TokenData<ValidatedClaims> =
            jsonwebtoken::decode(&self.0.expose_secret(), &decoding_key, &validation)
                .context("Failed to validate the signature of the CLI token")?;
        Ok(token.claims)
    }

    pub fn raw(&self) -> &Secret<String> {
        &self.0
    }
}

fn key_algo2algo(key_algorithm: KeyAlgorithm) -> Result<Algorithm, anyhow::Error> {
    match key_algorithm {
        KeyAlgorithm::HS256 => Ok(Algorithm::HS256),
        KeyAlgorithm::RS256 => Ok(Algorithm::RS256),
        KeyAlgorithm::ES256 => Ok(Algorithm::ES256),
        KeyAlgorithm::PS256 => Ok(Algorithm::PS256),
        KeyAlgorithm::HS384 => Ok(Algorithm::HS384),
        KeyAlgorithm::RS384 => Ok(Algorithm::RS384),
        KeyAlgorithm::ES384 => Ok(Algorithm::ES384),
        KeyAlgorithm::PS384 => Ok(Algorithm::PS384),
        KeyAlgorithm::HS512 => Ok(Algorithm::HS512),
        KeyAlgorithm::RS512 => Ok(Algorithm::RS512),
        KeyAlgorithm::PS512 => Ok(Algorithm::PS512),
        KeyAlgorithm::EdDSA => Ok(Algorithm::EdDSA),
        _ => Err(anyhow::anyhow!(
            "Unsupported key algorithm: {:?}",
            key_algorithm
        )),
    }
}

#[derive(serde::Deserialize)]
/// `ValidationClaims` can't be constructed outside of this module.
/// The only way to obtain one is via [`CliToken::validate`].
pub struct ValidatedClaims {
    #[serde(with = "time::serde::timestamp", rename = "iat")]
    issued_at: time::OffsetDateTime,
}

impl ValidatedClaims {
    /// Get the time at which the token was issued.
    pub fn issued_at(&self) -> &time::OffsetDateTime {
        &self.issued_at
    }
}
