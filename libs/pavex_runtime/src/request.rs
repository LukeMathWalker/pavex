use http::{Method, Uri, Version, HeaderMap};

#[non_exhaustive]
#[derive(Debug)]
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

