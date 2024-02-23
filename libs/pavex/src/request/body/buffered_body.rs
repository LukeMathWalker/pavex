use bytes::Bytes;
use http::header::CONTENT_LENGTH;
use http_body_util::{BodyExt, Limited};
use ubyte::ByteUnit;

use crate::blueprint::constructor::{Constructor, Lifecycle, RegisteredConstructor};
use crate::blueprint::Blueprint;
use crate::{f, request::body::errors::SizeLimitExceeded, request::RequestHead};

use super::{
    errors::{ExtractBufferedBodyError, UnexpectedBufferError},
    BodySizeLimit, RawIncomingBody,
};

#[derive(Debug)]
#[non_exhaustive]
/// Buffer the entire body of an incoming request in memory.
///
/// # Guide
///
/// `BufferedBody` is the ideal building block for _other_ extractors that need to
/// have the entire body available in memory to do their job (e.g. [`JsonBody`](super::JsonBody)).  
/// It can also be useful if you need to access the raw bytes of the body ahead of deserialization
/// (e.g. to compute its hash as a step of a signature verification process).  
///
/// Check out the ["Low-level access"](https://pavex.dev/docs/guide/request_data/body/byte_wrappers/)
/// section of Pavex's guide for a thorough introduction to `BufferedBody`.
///
/// # Security
///
/// `BufferedBody` includes a size limit to prevent denial-of-service attacks.
/// Check out [the guide](https://pavex.dev/docs/guide/request_data/body/byte_wrappers/#body-size-limit)
/// for examples on how to configure it.
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
            BodySizeLimit::Enabled { max_size } => {
                Self::_extract_with_limit(request_head, body, max_size).await
            }
            BodySizeLimit::Disabled => match body.collect().await {
                Ok(collected) => Ok(Self {
                    bytes: collected.to_bytes(),
                }),
                Err(e) => Err(UnexpectedBufferError { source: e.into() }.into()),
            },
        }
    }

    /// Register the [default constructor](Self::default_constructor)
    /// for [`BufferedBody`] with a [`Blueprint`].
    pub fn register(bp: &mut Blueprint) -> RegisteredConstructor {
        Self::default_constructor().register(bp)
    }

    /// The [default constructor](BufferedBody::extract)
    /// and [error handler](ExtractBufferedBodyError::into_response)
    /// for [`BufferedBody`].
    pub fn default_constructor() -> Constructor {
        Constructor::new(f!(super::BufferedBody::extract), Lifecycle::RequestScoped)
            .error_handler(f!(super::errors::ExtractBufferedBodyError::into_response))
    }

    async fn _extract_with_limit<B>(
        request_head: &RequestHead,
        body: B,
        max_size: ByteUnit,
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
            max_size,
            content_length,
        };

        // We first check the `Content-Length` header, if it exists, to see if the
        // "expected" size of the body is larger than the maximum size limit.
        // If it is, we return an error immediately.
        // This is a performance optimization: it allows us to short-circuit the
        // body reading process entirely rather than reading the body incrementally
        // until the limit is reached.
        if let Some(len) = content_length {
            if len > max_size {
                return Err(limit_error().into());
            }
        }

        // We saturate to `usize::MAX` if we happen to be on a platform where
        // `usize` is smaller than `u64` (e.g. 32-bit platforms).
        let max_n_bytes = max_size.as_u64().try_into().unwrap_or(usize::MAX);
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
    use ubyte::ToByteUnit;

    use crate::request::RequestHead;

    use super::{BufferedBody, Bytes};

    // No headers.
    fn dummy_request_head() -> RequestHead {
        RequestHead {
            method: http::Method::GET,
            target: "/".parse().unwrap(),
            version: http::Version::HTTP_11,
            headers: HeaderMap::new(),
        }
    }

    #[tokio::test]
    async fn error_if_body_above_size_limit_without_content_length() {
        let raw_body = vec![0; 1000];

        // Smaller than the size of the body.
        let max_n_bytes = 100.bytes();
        assert!(raw_body.len() > max_n_bytes.as_u64() as usize);

        let body = crate::response::body::raw::Full::new(Bytes::from(raw_body));
        let err = BufferedBody::_extract_with_limit(&dummy_request_head(), body, max_n_bytes)
            .await
            .unwrap_err();
        insta::assert_display_snapshot!(err, @"The request body is larger than the maximum size limit enforced by this server.");
        insta::assert_debug_snapshot!(err, @r###"
        SizeLimitExceeded(
            SizeLimitExceeded {
                max_size: ByteUnit(
                    100,
                ),
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
        let max_size = 100.bytes();
        let body = crate::response::body::raw::Full::new(Bytes::from(vec![0; 500]));
        request_head
            .headers
            .insert("Content-Length", "1000".parse().unwrap());

        // Act
        let err = BufferedBody::_extract_with_limit(&request_head, body, max_size)
            .await
            .unwrap_err();
        insta::assert_display_snapshot!(err, @"The request body is larger than the maximum size limit enforced by this server.");
        insta::assert_debug_snapshot!(err, @r###"
        SizeLimitExceeded(
            SizeLimitExceeded {
                max_size: ByteUnit(
                    100,
                ),
                content_length: Some(
                    1000,
                ),
            },
        )
        "###);
    }
}
