use crate::activation::{
    HTTP_CLIENT, InvalidWizardKey, MalformedWizardKey, json_api::JsonApiErrors,
};
use redact::Secret;

use super::{WizardKeyError, token::CliToken};

/// A short-lived key obtained via Pavex's Console.
///
/// It can be exchanged for a CLI token and an activation key.
pub struct WizardKey(Secret<String>);

impl WizardKey {
    pub fn new(key: Secret<String>) -> Self {
        Self(key)
    }

    /// Exchange the wizard key for a CLI token and an activation key.
    pub async fn exchange(&self) -> Result<(Secret<String>, CliToken), WizardKeyError> {
        #[derive(serde::Serialize)]
        struct Request {
            #[serde(serialize_with = "redact::expose_secret")]
            wizard_key: Secret<String>,
        }

        #[derive(serde::Deserialize)]
        struct Response {
            activation_key: Secret<String>,
            jwt: Secret<String>,
        }

        let response = HTTP_CLIENT
            .post("https://api.pavex.dev/v1/cli/wizard/key/exchange")
            .json(&Request {
                wizard_key: self.0.clone(),
            })
            .send()
            .await
            .map_err(|e| WizardKeyError::UnexpectedError(e.into()))?;

        if response.status().is_success() {
            let response: Response = response
                .json()
                .await
                .map_err(|e| WizardKeyError::UnexpectedError(e.into()))?;
            let cli_token = CliToken(response.jwt);
            Ok((response.activation_key, cli_token))
        } else if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            Err(InvalidWizardKey.into())
        } else if response.status() == reqwest::StatusCode::UNPROCESSABLE_ENTITY {
            let details = response
                .json::<JsonApiErrors>()
                .await
                .ok()
                .and_then(|body| {
                    body.errors.into_iter().find_map(|e| {
                        if e.code == "malformed_wizard_key" {
                            e.detail
                        } else {
                            None
                        }
                    })
                });
            Err(MalformedWizardKey { details }.into())
        } else {
            Err(WizardKeyError::UnexpectedError(
                response.error_for_status().unwrap_err().into(),
            ))
        }
    }
}
