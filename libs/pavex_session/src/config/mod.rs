//! Types related to [`SessionConfig`][crate::SessionConfig].
mod cookie;
mod state;

pub use cookie::{SessionCookieConfig, SessionCookieKind};
pub use state::{
    InvalidTtlExtensionThreshold, MissingServerState, ServerStateCreation, SessionStateConfig,
    TtlExtensionThreshold, TtlExtensionTrigger,
};
