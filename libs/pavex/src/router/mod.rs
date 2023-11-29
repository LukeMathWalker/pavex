//! Dispatch requests to the appropriate handler.
pub use allowed_methods::AllowedMethods;
pub use fallback::default_fallback;

mod allowed_methods;
mod fallback;
