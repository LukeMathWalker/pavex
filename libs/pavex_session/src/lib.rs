/*!
Session management for Pavex.

# Why do we need sessions?

The HTTP protocol, at a first glance, is stateless: the client sends a request, the server
parses its content, performs some processing and returns a response. The outcome is only
influenced by the provided inputs (i.e. the request content) and whatever state the server
queries while performing its processing.

Stateless systems are easier to reason about, but they are not quite as powerful as we need them
to be - e.g. how do you authenticate a user? The user would be forced to authenticate **for
every single request**. That is, for example, how 'Basic' Authentication works. While it may
work for a machine user (i.e. an API client), it is impractical for a person—you do not want a
login prompt on every single page you navigate to!

**Sessions** are the solution. They allow the server to attach state to a set of requests coming
from the same client. They are built on top of cookies: the server sets a
cookie in the HTTP response (`Set-Cookie` header), the client (e.g. the browser) stores the
cookie and sends it back to the server whenever it issues new requests (using the `Cookie` header).

# Anatomy of a session

A session cookie contains:

- A unique identifier for the session, called **session ID**.
- Application-specific data attached to the session, called **client-side session state**.

The session ID is used by the server to attach **server-side state** to the session.
Server-side state is stored away from the client, inside a **session storage backend**—a
SQL database (e.g. PostgreSQL), a cache (e.g. Redis), or any other persistent storage system.

## References

Further reading on sessions:
- [RFC 6265](https://datatracker.ietf.org/doc/html/rfc6265);
- [OWASP's session management cheat-sheet](https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html).
*/
pub mod config;
mod id;
mod incoming;
mod middleware;
mod session_;
mod store_;
pub(crate) mod wire;

pub use id::SessionId;
pub use incoming::IncomingSession;
pub use middleware::finalize_session;
pub use session_::Session;
pub use store_::SessionStore;

pub mod store {
    //! Types and traits related to [`SessionStore`][super::SessionStore].
    pub use crate::store_::errors;
    pub use crate::store_::{SessionRecord, SessionRecordRef, SessionStorageBackend};
}

pub mod state {
    //! Types to manipulate either the client-side or the server-side session state.
    pub use crate::session_::errors;
    pub use crate::session_::{ClientSessionState, ClientSessionStateMut, ServerSessionState};
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
/// Configure how sessions are managed.
///
/// The default configuration follows
/// [OWASP's guidelines for secure session management](https://github.com/OWASP/ASVS/blob/67726f1976a759c58a82669d0dad3b16b9c04ecc/4.0/en/0x12-V3-Session-management.md).
pub struct SessionConfig {
    #[serde(default)]
    /// Configure the session cookie.
    pub cookie: crate::config::SessionCookieConfig,
    #[serde(default)]
    /// Configure how the session state should behave.
    pub state: crate::config::SessionStateConfig,
}
