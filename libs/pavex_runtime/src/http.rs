//! Types related to the HTTP protocol (status codes, headers, etc).
//!
//! This module re-exports all the items from the [`http`] crate
//! that are needed to work with Pavex's [`RequestHead`](crate::request::RequestHead)s and
//! [`Response`](crate::response::Response)s.
pub use ::http::header;
pub use ::http::method;
pub use ::http::status;
pub use ::http::uri;
pub use ::http::version;

// Re-export commonly used types at the top-level for convenience.
pub use header::HeaderMap;
pub use header::HeaderName;
pub use header::HeaderValue;
pub use method::Method;
pub use status::StatusCode;
pub use uri::Uri;
pub use version::Version;
