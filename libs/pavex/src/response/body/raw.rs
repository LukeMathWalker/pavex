//! Low-level tools for building and manipulating [`Response`](crate::response::Response) bodies.
//!
//! Primarily useful if you are working with
//! [`Response::set_raw_body`](crate::response::Response::set_raw_body).
pub use bytes::{Bytes, BytesMut};
/// Trait representing a streaming [`Response`](crate::response::Response) body.  
///
pub use http_body::Body as RawBody;
pub use http_body_util::{Empty, Full};
