use crate::http::HeaderValue;
use bytes::{Bytes, BytesMut};
use http_body::Full;
use mime::{APPLICATION_JSON, TEXT_HTML_UTF_8};
use std::borrow::Cow;

pub trait TypedBody {
    type Body: http_body::Body<Data = Bytes> + Send + Sync + 'static;

    fn content_type(&self) -> HeaderValue;

    fn body(self) -> Self::Body;
}

pub struct Json(Bytes);

impl Json {
    pub fn new<T>(value: T) -> Result<Self, serde_json::Error>
    where
        T: serde::Serialize,
    {
        let bytes = serde_json::to_vec(&value)?;
        Ok(Self(bytes.into()))
    }
}

impl TypedBody for Json {
    type Body = Full<Bytes>;

    fn content_type(&self) -> HeaderValue {
        HeaderValue::from_static(APPLICATION_JSON.as_ref())
    }

    fn body(self) -> Self::Body {
        Full::new(self.0)
    }
}

pub struct Html(Bytes);

impl Html {
    pub fn from_string(s: String) -> Self {
        Self(s.into())
    }

    pub fn from_static(s: &'static str) -> Self {
        Self(Bytes::from_static(s.as_bytes()))
    }

    pub fn from_cow(s: Cow<'static, str>) -> Self {
        match s {
            Cow::Borrowed(s) => Self::from_static(s),
            Cow::Owned(s) => Self::from_string(s),
        }
    }
}

impl TypedBody for Html {
    type Body = Full<Bytes>;

    fn content_type(&self) -> HeaderValue {
        HeaderValue::from_static(TEXT_HTML_UTF_8.as_ref())
    }

    fn body(self) -> Self::Body {
        Full::new(self.0)
    }
}

impl TypedBody for String {
    type Body = Full<Bytes>;

    fn content_type(&self) -> HeaderValue {
        HeaderValue::from_static(mime::TEXT_PLAIN_UTF_8.as_ref())
    }

    fn body(self) -> Self::Body {
        Full::new(self.into())
    }
}

impl TypedBody for &'static str {
    type Body = Full<Bytes>;

    fn content_type(&self) -> HeaderValue {
        HeaderValue::from_static(mime::TEXT_PLAIN_UTF_8.as_ref())
    }

    fn body(self) -> Self::Body {
        Full::new(self.into())
    }
}

impl TypedBody for Cow<'static, str> {
    type Body = Full<Bytes>;

    fn content_type(&self) -> HeaderValue {
        HeaderValue::from_static(mime::TEXT_PLAIN_UTF_8.as_ref())
    }

    fn body(self) -> Self::Body {
        match self {
            Cow::Borrowed(s) => s.body(),
            Cow::Owned(s) => s.body(),
        }
    }
}

impl TypedBody for Bytes {
    type Body = Full<Bytes>;

    fn content_type(&self) -> HeaderValue {
        HeaderValue::from_static(mime::APPLICATION_OCTET_STREAM.as_ref())
    }

    fn body(self) -> Self::Body {
        Full::new(self)
    }
}

impl TypedBody for Vec<u8> {
    type Body = Full<Bytes>;

    fn content_type(&self) -> HeaderValue {
        HeaderValue::from_static(mime::APPLICATION_OCTET_STREAM.as_ref())
    }

    fn body(self) -> Self::Body {
        Full::new(self.into())
    }
}

impl TypedBody for &'static [u8] {
    type Body = Full<Bytes>;

    fn content_type(&self) -> HeaderValue {
        HeaderValue::from_static(mime::APPLICATION_OCTET_STREAM.as_ref())
    }

    fn body(self) -> Self::Body {
        Full::new(self.into())
    }
}

impl TypedBody for BytesMut {
    type Body = Full<Bytes>;

    fn content_type(&self) -> HeaderValue {
        HeaderValue::from_static(mime::APPLICATION_OCTET_STREAM.as_ref())
    }

    fn body(self) -> Self::Body {
        Full::new(self.freeze())
    }
}

impl TypedBody for Cow<'static, [u8]> {
    type Body = Full<Bytes>;

    fn content_type(&self) -> HeaderValue {
        HeaderValue::from_static(mime::APPLICATION_OCTET_STREAM.as_ref())
    }

    fn body(self) -> Self::Body {
        match self {
            Cow::Borrowed(s) => s.body(),
            Cow::Owned(s) => s.body(),
        }
    }
}
