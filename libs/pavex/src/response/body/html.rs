use super::TypedBody;
use crate::http::HeaderValue;
use bytes::Bytes;
use http_body::Full;
use mime::TEXT_HTML_UTF_8;
use std::borrow::Cow;

/// A [`Response`](crate::response::Response) body with `Content-Type` set to
/// `text/html; charset=utf-8`.
///
/// # Example
///
/// ```rust
/// use pavex::response::{Response, body::Html};
/// use pavex::http::header::CONTENT_TYPE;
///
/// let html: Html = r#"<body>
///     <h1>Hey there!</h1>
/// </body>"#.into();
/// let response = Response::ok().set_typed_body(html);
///
/// assert_eq!(response.headers()[CONTENT_TYPE], "text/html; charset=utf-8");
/// ```
pub struct Html(Bytes);

impl From<String> for Html {
    fn from(s: String) -> Self {
        Self(s.into())
    }
}

impl From<&'static str> for Html {
    fn from(s: &'static str) -> Self {
        Self(Bytes::from_static(s.as_bytes()))
    }
}

impl From<Cow<'static, str>> for Html {
    fn from(s: Cow<'static, str>) -> Self {
        match s {
            Cow::Borrowed(s) => s.into(),
            Cow::Owned(s) => s.into(),
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
