//! Types related to the HTTP protocol (status codes, headers, etc).
//! 
//! This module re-exports most of the [`http`] crate, with a few exceptions.
pub use ::http::header as header;
pub use ::http::method as method;
pub use ::http::status as status;
pub use ::http::uri as uri;
pub use ::http::version as version;

// Re-export commonly used types at the top-level for convenience.
pub use header::HeaderMap;
pub use header::HeaderName;
pub use header::HeaderValue;
pub use method::Method;
pub use status::StatusCode;
pub use uri::Uri;
pub use version::Version;