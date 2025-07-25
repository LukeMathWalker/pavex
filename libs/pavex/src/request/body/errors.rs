//! Errors that can occur while extracting information from the request body.
use crate::Response;
use pavex_macros::methods;
use ubyte::ByteUnit;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
/// The error returned by [`JsonBody::extract`] when the extraction fails.
///
/// [`JsonBody::extract`]: crate::request::body::json::JsonBody::extract
pub enum ExtractJsonBodyError {
    #[error(transparent)]
    /// See [`MissingJsonContentType`] for details.
    MissingContentType(#[from] MissingJsonContentType),
    #[error(transparent)]
    /// See [`JsonContentTypeMismatch`] for details.
    ContentTypeMismatch(#[from] JsonContentTypeMismatch),
    #[error(transparent)]
    /// See [`JsonDeserializationError`] for details.
    DeserializationError(#[from] JsonDeserializationError),
}

#[methods]
impl ExtractJsonBodyError {
    /// Convert an [`ExtractJsonBodyError`] into an HTTP response.
    #[error_handler(pavex = crate)]
    pub fn into_response(&self) -> Response {
        match self {
            ExtractJsonBodyError::MissingContentType(_)
            | ExtractJsonBodyError::ContentTypeMismatch(_) => Response::unsupported_media_type(),
            ExtractJsonBodyError::DeserializationError(_) => Response::bad_request(),
        }
        .set_typed_body(format!("{self}"))
    }
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
/// The error returned by [`BufferedBody::extract`] when the extraction fails.
///
/// [`BufferedBody::extract`]: crate::request::body::buffered_body::BufferedBody::extract
pub enum ExtractBufferedBodyError {
    #[error(transparent)]
    /// See [`SizeLimitExceeded`] for details.
    SizeLimitExceeded(#[from] SizeLimitExceeded),
    #[error(transparent)]
    /// See [`UnexpectedBufferError`] for details.
    UnexpectedBufferError(#[from] UnexpectedBufferError),
}

#[methods]
impl ExtractBufferedBodyError {
    /// Convert an [`ExtractBufferedBodyError`] into an HTTP response.
    #[error_handler(pavex = crate)]
    pub fn into_response(&self) -> Response {
        match self {
            ExtractBufferedBodyError::SizeLimitExceeded(_) => Response::payload_too_large(),
            ExtractBufferedBodyError::UnexpectedBufferError(_) => Response::internal_server_error(),
        }
        .set_typed_body(format!("{self}"))
    }
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
/// The error returned by [`UrlEncodedBody::extract`] when the extraction fails.
///
/// [`UrlEncodedBody::extract`]: crate::request::body::url_encoded::UrlEncodedBody::extract
pub enum ExtractUrlEncodedBodyError {
    #[error(transparent)]
    /// See [`MissingUrlEncodedContentType`] for details.
    MissingContentType(#[from] MissingUrlEncodedContentType),
    #[error(transparent)]
    /// See [`UrlEncodedContentTypeMismatch`] for details.
    ContentTypeMismatch(#[from] UrlEncodedContentTypeMismatch),
    #[error(transparent)]
    /// See [`UrlEncodedBodyDeserializationError`] for details.
    DeserializationError(#[from] UrlEncodedBodyDeserializationError),
}

#[methods]
impl ExtractUrlEncodedBodyError {
    /// Convert an [`ExtractUrlEncodedBodyError`] into an HTTP response.
    #[error_handler(pavex = crate)]
    pub fn into_response(&self) -> Response {
        match self {
            ExtractUrlEncodedBodyError::MissingContentType(_)
            | ExtractUrlEncodedBodyError::ContentTypeMismatch(_) => {
                Response::unsupported_media_type()
            }
            ExtractUrlEncodedBodyError::DeserializationError(_) => Response::bad_request(),
        }
        .set_typed_body(format!("{self}"))
    }
}

#[derive(Debug, thiserror::Error)]
#[error("The request body is larger than the maximum size limit enforced by this server.")]
#[non_exhaustive]
/// The request body is larger than the maximum size limit enforced by this server.
pub struct SizeLimitExceeded {
    /// The maximum size limit enforced by this server.
    pub max_size: ByteUnit,
    /// The value of the `Content-Length` header for the request that breached the body
    /// size limit.
    ///
    /// It's set to `None` if the `Content-Length` header was missing or invalid.
    /// If it's set to `Some(n)` and `n` is smaller than `max_n_bytes`, then the request
    /// lied about the size of its body in the `Content-Length` header.
    pub content_length: Option<usize>,
}

#[derive(Debug, thiserror::Error)]
#[error("Something went wrong while reading the request body.")]
#[non_exhaustive]
/// Something went wrong while reading the request body, but we don't know what specifically.
pub struct UnexpectedBufferError {
    #[source]
    pub(super) source: Box<dyn std::error::Error + Send + Sync>,
}

#[derive(Debug, thiserror::Error)]
#[error(
    "The `Content-Type` header is missing. This endpoint expects requests with a `Content-Type` header set to `application/json`, or another `application/*+json` MIME type"
)]
#[non_exhaustive]
/// The `Content-Type` header is missing, while we expected it to be set to `application/json`, or
/// another `application/*+json` MIME type.
pub struct MissingJsonContentType;

#[derive(Debug, thiserror::Error)]
#[error("Failed to deserialize the body as a JSON document.\n{source}")]
#[non_exhaustive]
/// Something went wrong when deserializing the request body into the specified type.
pub struct JsonDeserializationError {
    #[source]
    pub(super) source: serde_path_to_error::Error<serde_json::Error>,
}

#[derive(Debug, thiserror::Error)]
#[error(
    "The `Content-Type` header was set to `{actual}`. This endpoint expects requests with a `Content-Type` header set to `application/json`, or another `application/*+json` MIME type"
)]
#[non_exhaustive]
/// The `Content-Type` header not set to `application/json`, or another `application/*+json` MIME type.
pub struct JsonContentTypeMismatch {
    /// The actual value of the `Content-Type` header for this request.
    pub actual: String,
}

#[derive(Debug, thiserror::Error)]
#[error(
    "The `Content-Type` header is missing. This endpoint expects requests with a `Content-Type` header set to `application/x-www-form-urlencoded`"
)]
#[non_exhaustive]
/// The `Content-Type` header is missing, while we expected it to be set to `application/x-www-form-urlencoded`.
pub struct MissingUrlEncodedContentType;

#[derive(Debug, thiserror::Error)]
#[error(
    "The `Content-Type` header was set to `{actual}`. This endpoint expects requests with a `Content-Type` header set to `application/x-www-form-urlencoded`"
)]
#[non_exhaustive]
/// The `Content-Type` header not set to `application/x-www-form-urlencoded`.
pub struct UrlEncodedContentTypeMismatch {
    /// The actual value of the `Content-Type` header for this request.
    pub actual: String,
}

#[derive(Debug, thiserror::Error)]
#[error("Failed to deserialize the body as a urlencoded form.\n{source}")]
#[non_exhaustive]
/// Something went wrong when deserializing the request body into the specified type.
pub struct UrlEncodedBodyDeserializationError {
    #[source]
    pub(super) source: serde_html_form::de::Error,
}
