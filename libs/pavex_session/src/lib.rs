/*!
Session management for Pavex.

Check out the [session guide in Pavex's documentation](https://pavex.dev/guide/sessions/) for a thorough introduction to sessions
and how to use them in your application.
*/
pub mod config;
mod id;
mod incoming;
mod middleware;
mod session_;
mod store_;
pub(crate) mod wire;

use std::collections::HashMap;

pub use id::SessionId;
pub use incoming::IncomingSession;
pub use middleware::{FINALIZE_SESSION, finalize_session};
use pavex::transient;
pub use session_::Session;
pub use store_::SessionStore;

pub mod store {
    //! Types and traits related to [`SessionStore`][super::SessionStore].
    pub use crate::store_::errors;
    pub use crate::store_::{SessionRecord, SessionRecordRef, SessionStorageBackend};
}

pub use crate::session_::errors;

pub mod client {
    //! Types to manipulate the client-side session state.
    pub use crate::session_::{ClientSessionState, ClientSessionStateMut};
}

/// A convenient alias for the shape of the session state.
pub(crate) type State = HashMap<std::borrow::Cow<'static, str>, serde_json::Value>;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
/// Configure how sessions are managed.
///
/// The default configuration follows
/// [OWASP's guidelines for secure session management](https://github.com/OWASP/ASVS/blob/67726f1976a759c58a82669d0dad3b16b9c04ecc/4.0/en/0x12-V3-Session-management.md).
#[pavex::config(key = "session")]
pub struct SessionConfig {
    #[serde(default)]
    /// Configure the session cookie.
    pub cookie: crate::config::SessionCookieConfig,
    #[serde(default)]
    /// Configure how the session state should behave.
    pub state: crate::config::SessionStateConfig,
}

impl SessionConfig {
    /// Create a new session configuration with the default settings.
    pub fn new() -> Self {
        Self::default()
    }

    #[doc(hidden)]
    #[transient]
    pub fn cookie_config(&self) -> &crate::config::SessionCookieConfig {
        &self.cookie
    }

    #[doc(hidden)]
    #[transient]
    pub fn state_config(&self) -> &crate::config::SessionStateConfig {
        &self.state
    }
}
