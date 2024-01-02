use http::{HeaderMap, Method, Uri, Version};

#[non_exhaustive]
#[derive(Debug)]
/// All the information that is transmitted as part of an HTTP request ahead of the body.
///
/// It includes the [method](Method), the [URI](Uri),
/// the [HTTP version](Version), and the [headers](HeaderMap).
///
/// # Framework primitive
///
/// `RequestHead` is a framework primitive—you don't need to register any constructor
/// with [`Blueprint`] to use it in your application.
///
/// [`Blueprint`]: crate::blueprint::Blueprint
pub struct RequestHead {
    pub method: Method,
    pub target: Uri,
    pub version: Version,
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
