use crate::{config::SessionCookieConfig, wire::WireClientState, SessionId};
use pavex::cookie::RequestCookies;
use pavex_tracing::fields::{error_details, error_message, ERROR_DETAILS, ERROR_MESSAGE};
use serde_json::Value;
use std::collections::HashMap;

/// The session information attached to the incoming request.
///
/// Built using [`IncomingSession::extract`].
pub struct IncomingSession {
    pub(crate) id: SessionId,
    pub(crate) client_state: HashMap<String, Value>,
}

impl IncomingSession {
    /// Extract a session cookie from the incoming request, if it exists.
    ///
    /// If the cookie is not found, or if the cookie is invalid, this method will return `None`.
    pub fn extract<'server, 'request, 'cookie>(
        cookies: &'request RequestCookies<'cookie>,
        config: &'server SessionCookieConfig,
    ) -> Option<Self> {
        let cookie = cookies.get(&config.name)?;
        match serde_json::from_str::<WireClientState>(cookie.value()) {
            Ok(s) => Some(Self {
                id: s.session_id,
                client_state: s.user_values.into_owned(),
            }),
            Err(e) => {
                tracing::event!(
                    tracing::Level::WARN,
                    { ERROR_MESSAGE } = error_message(&e),
                    { ERROR_DETAILS } = error_details(&e),
                    "Invalid client state for session, creating a new session."
                );
                None
            }
        }
    }
}
