//! Dispatch requests to the appropriate handler.
pub use allowed_methods::{AllowedMethods, MethodAllowList};
pub use fallback::{DEFAULT_FALLBACK, default_fallback};

mod allowed_methods;
mod fallback;
