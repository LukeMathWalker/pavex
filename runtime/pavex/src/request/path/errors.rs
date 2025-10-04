//! Errors that can happen when extracting path parameters.
use std::str::Utf8Error;

use pavex_macros::methods;

use crate::Response;

/// The error returned by [`PathParams::extract`] when the extraction fails.
///
/// See [`PathParams::extract`] and the documentation of each error variant for more details.
///
/// Pavex provides [`ExtractPathParamsError::into_response`] as the default error handler for
/// this failure.
///
/// [`PathParams::extract`]: crate::request::path::PathParams::extract
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ExtractPathParamsError {
    #[error(transparent)]
    /// See [`InvalidUtf8InPathParam`] for details.
    InvalidUtf8InPathParameter(InvalidUtf8InPathParam),
    #[error(transparent)]
    /// See [`PathDeserializationError`] for details.
    PathDeserializationError(PathDeserializationError),
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
/// One of the percent-decoded path parameters is not a valid UTF8 string.
///
/// URL parameters must be percent-encoded whenever they contain characters that are not
/// URL safe—e.g. whitespaces.
///
/// Pavex automatically percent-decodes URL parameters before trying to deserialize them
/// in [`PathParams<T>`].
/// This error is returned whenever the percent-decoding step fails—i.e. the decoded data is not a
/// valid UTF8 string.
///
/// # Example
///
/// One of our routes is `/address/{address_id}`.
/// We receive a request with `/address/the%20street` as path—`address_id` is set to
/// `the%20street` and Pavex automatically decodes it into `the street`.
///
/// We could also receive a request using `/address/dirty%DE~%C7%1FY` as path—`address_id`, when
/// decoded, is a sequence of bytes that cannot be interpreted as a well-formed UTF8 string.
/// This error is then returned.
///
/// [`PathParams<T>`]: struct@crate::request::path::PathParams
#[error(
    "`{invalid_raw_segment}` cannot be used as `{invalid_key}` \
since it is not a well-formed UTF8 string when percent-decoded"
)]
pub struct InvalidUtf8InPathParam {
    pub(super) invalid_key: String,
    pub(super) invalid_raw_segment: String,
    #[source]
    pub(super) source: Utf8Error,
}

/// The error returned by [`EncodedParamValue::decode`] when the percent-decoded path parameter
/// is not a valid UTF8 string.
///
/// Path parameters must be percent-encoded whenever they contain characters that are not
/// URL safe—e.g. whitespaces.
/// This error is returned whenever the percent-decoding step fails—i.e. the decoded data is not a
/// valid UTF8 string.
///
/// # Example
///
/// You might try to percent-decode `dirty%DE~%C7%1FY`.
/// When decoded, it is a sequence of bytes that cannot be interpreted as a well-formed UTF8 string.
/// This error is then returned.
///
/// [`EncodedParamValue::decode`]: super::EncodedParamValue::decode
#[derive(Debug, thiserror::Error)]
#[error("`{invalid_raw_segment}` is not a well-formed UTF8 string when percent-decoded")]
pub struct DecodeError {
    pub(super) invalid_raw_segment: String,
    #[source]
    pub(super) source: Utf8Error,
}

#[methods]
impl ExtractPathParamsError {
    /// Convert an [`ExtractPathParamsError`] into an HTTP response.
    ///
    /// It returns a `500 Internal Server Error` to the caller if the failure was caused by a
    /// programmer error (e.g. `T` in [`PathParams<T>`] is an unsupported type).
    /// It returns a `400 Bad Request` for all other cases.
    ///
    /// [`PathParams<T>`]: struct@crate::request::path::PathParams
    #[error_handler(pavex = crate)]
    pub fn into_response(&self) -> Response {
        match self {
            ExtractPathParamsError::InvalidUtf8InPathParameter(e) => {
                Response::bad_request().set_typed_body(format!("Invalid URL.\n{e}"))
            }
            ExtractPathParamsError::PathDeserializationError(e) => match e.kind {
                ErrorKind::ParseErrorAtKey { .. } | ErrorKind::ParseError { .. } => {
                    Response::bad_request().set_typed_body(format!("Invalid URL.\n{}", e.kind))
                }
                // We put the "custom" message variant here as well because it's not clear
                // whether it's a programmer error or not. We err on the side of safety and
                // prefer to return a 500 with an opaque error message.
                ErrorKind::Message(_) | ErrorKind::UnsupportedType { .. } => {
                    Response::internal_server_error()
                        .set_typed_body("Something went wrong when trying to process the request")
                }
            },
        }
    }
}

#[derive(Debug)]
/// Something went wrong when trying to deserialize the percent-decoded URL parameters into
/// the target type you specified—`T` in [`PathParams<T>`].
///
/// You can use [`PathDeserializationError::kind`] to get more details about the error.
///
/// [`PathParams<T>`]: struct@crate::request::path::PathParams
pub struct PathDeserializationError {
    pub(super) kind: ErrorKind,
}

impl PathDeserializationError {
    pub(super) fn new(kind: ErrorKind) -> Self {
        Self { kind }
    }

    /// Retrieve the details of the error that occurred while trying to deserialize the URL
    /// parameters into the target type.
    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }

    #[track_caller]
    pub(super) fn unsupported_type(name: &'static str) -> Self {
        Self::new(ErrorKind::UnsupportedType { name })
    }
}

impl serde::de::Error for PathDeserializationError {
    #[inline]
    fn custom<T>(msg: T) -> Self
    where
        T: std::fmt::Display,
    {
        Self {
            kind: ErrorKind::Message(msg.to_string()),
        }
    }
}

impl std::fmt::Display for PathDeserializationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.kind, f)
    }
}

impl std::error::Error for PathDeserializationError {}

/// The kinds of errors that can happen when deserializing into a [`PathParams`].
///
/// This type is obtained through [`PathDeserializationError::kind`] and is useful for building
/// more precise error messages (e.g. implementing your own custom conversion from
/// [`PathDeserializationError`] into an HTTP response).
///
/// [`PathParams`]: struct@crate::request::path::PathParams
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ErrorKind {
    /// Failed to parse the value at a specific key into the expected type.
    ///
    /// This variant is used when deserializing into types that have named fields, such as structs.
    ParseErrorAtKey {
        /// The key at which the value was located.
        key: String,
        /// The value from the URI.
        value: String,
        /// The expected type of the value.
        expected_type: &'static str,
    },

    /// Failed to parse a value into the expected type.
    ///
    /// This variant is used when deserializing into a primitive type (such as `String` and `u32`).
    ParseError {
        /// The value from the URI.
        value: String,
        /// The expected type of the value.
        expected_type: &'static str,
    },

    /// Tried to serialize into an unsupported type such as collections, tuples or nested maps.
    ///
    /// This error kind is caused by programmer errors and thus gets converted into a `500 Internal
    /// Server Error` response.
    UnsupportedType {
        /// The name of the unsupported type.
        name: &'static str,
    },

    /// Catch-all variant for errors that don't fit any other variant.
    Message(String),
}

impl std::fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorKind::Message(error) => std::fmt::Display::fmt(error, f),
            ErrorKind::UnsupportedType { name } => {
                write!(
                    f,
                    "`{name}` is not a supported type for the `PathParams` extractor. \
                    The type `T` in `Path<T>` must be a struct (with one public field for each \
                    templated path segment) or a map (e.g. `HashMap<&'a str, Cow<'a, str>>`)."
                )
            }
            ErrorKind::ParseErrorAtKey {
                key,
                value,
                expected_type,
            } => write!(
                f,
                "`{key}` is set to `{value}`, which we can't parse as a `{expected_type}`"
            ),
            ErrorKind::ParseError {
                value,
                expected_type,
            } => write!(f, "We can't parse `{value}` as a `{expected_type}`"),
        }
    }
}
