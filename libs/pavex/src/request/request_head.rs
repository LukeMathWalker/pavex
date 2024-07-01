use http::{HeaderMap, Method, Uri, Version};

#[derive(Debug)]
/// All the information that is transmitted as part of an HTTP request ahead of the body.
///
/// It includes the [method](Method), the [target](Uri),
/// the [HTTP version](Version), and the [headers](HeaderMap).
///
/// # Guide
///
/// Check out [the guide](https://pavex.dev/docs/guide/request_data/wire_data/)
/// for a thorough introduction to `RequestHead`.
pub struct RequestHead {
    /// The HTTP method of the request.
    pub method: Method,
    /// The [target](https://datatracker.ietf.org/doc/html/rfc7230#section-5.3) of the request.
    pub target: Uri,
    /// The HTTP version used by the request.
    pub version: Version,
    /// The headers attached to the request.
    pub headers: HeaderMap,
}

impl From<http::request::Parts> for RequestHead {
    fn from(parts: http::request::Parts) -> Self {
        Self {
            method: parts.method,
            target: parts.uri,
            version: parts.version,
            headers: parts.headers,
        }
    }
}
