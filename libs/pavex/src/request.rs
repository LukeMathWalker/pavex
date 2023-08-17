//! An incoming HTTP request.
use http::{HeaderMap, Method, Uri, Version};

#[non_exhaustive]
#[derive(Debug)]
/// All the information that is transmitted as part of an HTTP request ahead of the body.
///
/// It includes the [method](Method), the [URI](Uri),
/// the [HTTP version](Version), and the [headers](HeaderMap).
pub struct RequestHead {
    pub method: Method,
    pub uri: Uri,
    pub version: Version,
    pub headers: HeaderMap,
}

impl From<http::request::Parts> for RequestHead {
    fn from(parts: http::request::Parts) -> Self {
        Self {
            method: parts.method,
            uri: parts.uri,
            version: parts.version,
            headers: parts.headers,
        }
    }
}
