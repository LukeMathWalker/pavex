use std::borrow::Cow;

use bytes::{Bytes, BytesMut};
use http_body_util::Full;

use crate::http::HeaderValue;

use super::TypedBody;

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
