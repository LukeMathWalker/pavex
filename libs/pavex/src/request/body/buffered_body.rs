use bytes::Bytes;
use http::header::CONTENT_LENGTH;
use http_body_util::{BodyExt, Limited};

use crate::{request::body::errors::SizeLimitExceeded, request::RequestHead};

use super::{
    errors::{ExtractBufferedBodyError, UnexpectedBufferError},
    BodySizeLimit, RawIncomingBody,
};

#[derive(Debug)]
#[non_exhaustive]
/// Buffer the entire body of an incoming request in memory.
///
/// `BufferedBody` is the ideal building block for _other_ extractors that need to
/// have the entire body available in memory to do their job (e.g. [`JsonBody`](super::JsonBody)).  
///
/// It can also be useful if you need to access the raw bytes of the body ahead of deserialization
/// (e.g. to compute its hash as a step of a signature verification process).
///
/// # Sections
///
/// - [Example](#example)
/// - [Installation](#installtion)
/// - [Body size limit](#body-size-limit)
///
/// # Example
///
/// ```rust
/// use pavex::http::StatusCode;
/// use pavex::request::body::BufferedBody;
///
/// // The `BufferedBody` extractor consumes the raw request body stream
/// // and buffers its entire contents in memory.
/// pub fn handler(body: &BufferedBody) -> StatusCode {
///     format!(
///         "The incoming request contains {} bytes",
///         body.bytes.len(),
///     );
///     // [...]
/// #    StatusCode::OK
/// }
/// ```
///
/// # Installation
///
/// You need the register the default constructor and error handler for
/// `BufferedBody` in your `Blueprint`:
///
/// ```rust
/// use pavex::f;
/// use pavex::blueprint::{Blueprint, constructor::Lifecycle};
///
/// fn blueprint() -> Blueprint {
///    let mut bp = Blueprint::new();
///    // Register the default constructor and error handler for `BufferedBody`.
///    bp.constructor(
///         f!(pavex::request::body::BufferedBody::extract),
///         Lifecycle::RequestScoped,
///     ).error_handler(
///         f!(pavex::request::body::errors::ExtractBufferedBodyError::into_response)
///     );
///     // [...]
///     bp
/// }
/// ```
///
/// You can then use the `BufferedBody` extractor as input to your route handlers and constructors.
///
/// # Body size limit
///
/// To prevent denial-of-service attacks, Pavex enforces an upper limit on the body size when
/// trying to buffer it in memory. The default limit is 2 MBs.  
///
/// [`BufferedBody::extract`] will return the [`SizeLimitExceeded`](ExtractBufferedBodyError::SizeLimitExceeded) error variant if the limit is exceeded.
///
/// You can customize the limit by registering a constructor for [`BodySizeLimit`] in
/// your `Blueprint`:
///
/// ```rust
/// use pavex::f;
/// use pavex::blueprint::{Blueprint, constructor::Lifecycle};
/// use pavex::request::body::BodySizeLimit;
///
/// pub fn body_size_limit() -> BodySizeLimit {
///     BodySizeLimit::Enabled {
///         max_n_bytes: 10_485_760 // 10 MBs
///     }
/// }
///
/// fn blueprint() -> Blueprint {
///     let mut bp = Blueprint::new();
///     // Register a custom constructor for `BodySizeLimit`.
///     bp.constructor(f!(crate::body_size_limit), Lifecycle::Singleton);
///     // [...]
///     bp
/// }
/// ```
///
/// You can also disable the limit entirely:  
///
/// ```rust
/// use pavex::f;
/// use pavex::blueprint::{Blueprint, constructor::Lifecycle};
/// use pavex::request::body::BodySizeLimit;
///
/// pub fn body_size_limit() -> BodySizeLimit {
///    BodySizeLimit::Disabled
/// }
///
/// fn blueprint() -> Blueprint {
///     let mut bp = Blueprint::new();
///     // Register a custom constructor for `BodySizeLimit`.
///     bp.constructor(f!(crate::body_size_limit), Lifecycle::Singleton);
///     // [...]
///     bp
/// }
/// ```
///
/// There might be situations where you want granular control instead of having
/// a single global limit for all incoming requests.  
/// You can leverage nesting for this purpose:
///
/// ```rust
/// use pavex::f;
/// use pavex::blueprint::{Blueprint, constructor::Lifecycle, router::{GET, POST}};
/// use pavex::request::body::BodySizeLimit;
/// # pub fn home() -> String { todo!() }
/// # pub fn upload() -> String { todo!() }
///
/// fn blueprint() -> Blueprint {
///     let mut bp = Blueprint::new();
///     bp.route(GET, "/", f!(crate::home));
///     bp.nest(upload_bp());
///     // [...]
///     bp
/// }
///
/// fn upload_bp() -> Blueprint {
///     let mut bp = Blueprint::new();
///     // This limit will only apply to the routes registered
///     // in this nested blueprint.
///     bp.constructor(f!(crate::body_size_limit), Lifecycle::Singleton);
///     bp.route(POST, "/upload", f!(crate::upload));
///     bp
/// }
///
/// pub fn upload_size_limit() -> BodySizeLimit {
///     BodySizeLimit::Enabled {
///         max_n_bytes: 1_073_741_824 // 1GB
///     }
/// }
/// ```
///
/// Check out `Blueprint::nest` and `Blueprint::nest_at` for more details on nesting.
pub struct BufferedBody {
    /// The buffer of bytes that represents the body of the incoming request.
    pub bytes: Bytes,
}

impl BufferedBody {
    /// Default constructor for [`BufferedBody`].
    ///
    /// If extraction fails, an [`ExtractBufferedBodyError`] is returned.
    pub async fn extract(
        request_head: &RequestHead,
        body: RawIncomingBody,
        body_size_limit: BodySizeLimit,
    ) -> Result<Self, ExtractBufferedBodyError> {
        match body_size_limit {
            BodySizeLimit::Enabled { max_n_bytes } => {
                Self::_extract_with_limit(request_head, body, max_n_bytes).await
            }
            BodySizeLimit::Disabled => match body.collect().await {
                Ok(collected) => Ok(Self {
                    bytes: collected.to_bytes(),
                }),
                Err(e) => Err(UnexpectedBufferError { source: e.into() }.into()),
            },
        }
    }

    async fn _extract_with_limit<B>(
        request_head: &RequestHead,
        body: B,
        max_n_bytes: usize,
    ) -> Result<Self, ExtractBufferedBodyError>
    where
        B: hyper::body::Body,
        B::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    {
        let content_length = request_head
            .headers
            .get(CONTENT_LENGTH)
            .and_then(|value| value.to_str().ok()?.parse::<usize>().ok());

        // Little shortcut to create a `SizeLimitExceeded` error.
        let limit_error = || SizeLimitExceeded {
            max_n_bytes,
            content_length,
        };

        // We first check the `Content-Length` header, if it exists, to see if the
        // "expected" size of the body is larger than the maximum size limit.
        // If it is, we return an error immediately.
        // This is a performance optimization: it allows us to short-circuit the
        // body reading process entirely rather than reading the body incrementally
        // until the limit is reached.
        if let Some(len) = content_length {
            if len > max_n_bytes {
                return Err(limit_error().into());
            }
        }

        // If the `Content-Length` header is missing, or if the expected size of the body
        // is smaller than the maximum size limit, we start buffering the body while keeping
        // track of the size limit.
        let limited_body = Limited::new(body, max_n_bytes);
        match limited_body.collect().await {
            Ok(collected) => Ok(Self {
                bytes: collected.to_bytes(),
            }),
            Err(e) => {
                if e.downcast_ref::<http_body_util::LengthLimitError>()
                    .is_some()
                {
                    Err(limit_error().into())
                } else {
                    Err(UnexpectedBufferError { source: e }.into())
                }
            }
        }
    }
}

impl From<BufferedBody> for Bytes {
    fn from(buffered_body: BufferedBody) -> Self {
        buffered_body.bytes
    }
}

#[cfg(test)]
mod tests {
    use http::HeaderMap;

    use crate::request::RequestHead;

    // No headers.
    fn dummy_request_head() -> RequestHead {
        RequestHead {
            method: http::Method::GET,
            uri: "/".parse().unwrap(),
            version: http::Version::HTTP_11,
            headers: HeaderMap::new(),
        }
    }

    #[tokio::test]
    async fn error_if_body_above_size_limit_without_content_length() {
        let body = Full::new(Bytes::from(vec![0; 1000]));
        // Smaller than the size of the body.
        let max_n_bytes = 100;
        let err = BufferedBody::_extract_with_limit(&dummy_request_head(), body, max_n_bytes)
            .await
            .unwrap_err();
        insta::assert_display_snapshot!(err, @"The request body is larger than the maximum size limit enforced by this server.");
        insta::assert_debug_snapshot!(err, @r###"
        SizeLimitExceeded(
            SizeLimitExceeded {
                max_n_bytes: 100,
                content_length: None,
            },
        )
        "###);
    }

    #[tokio::test]
    /// This is a case of a request lying about the size of its body,
    /// triggering the limit check even though the actual body size
    /// would have been fine.
    async fn error_if_content_length_header_is_larger_than_limit() {
        let mut request_head = dummy_request_head();

        // Smaller than the value declared in the `Content-Length` header,
        // even though it's bigger than the actual size of the body.
        let max_n_bytes = 100;
        let body = Full::new(Bytes::from(vec![0; 500]));
        request_head
            .headers
            .insert("Content-Length", "1000".parse().unwrap());

        // Act
        let err = BufferedBody::_extract_with_limit(&request_head, body, max_n_bytes)
            .await
            .unwrap_err();
        insta::assert_display_snapshot!(err, @"The request body is larger than the maximum size limit enforced by this server.");
        insta::assert_debug_snapshot!(err, @r###"
        SizeLimitExceeded(
            SizeLimitExceeded {
                max_n_bytes: 100,
                content_length: Some(
                    1000,
                ),
            },
        )
        "###);
    }
}
