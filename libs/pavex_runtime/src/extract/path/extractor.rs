use std::fmt::Debug;
use std::str::Utf8Error;

use http::StatusCode;
use matchit::Params;
use percent_encoding::percent_decode_str;
use serde::Deserialize;

use crate::extract::path::deserializer::PathDeserializer;
use crate::response::Response;

/// Extract (typed) path parameters from the incoming request.
///
/// # Example
///
/// ```rust
/// # use pavex_builder::{f, router::GET, Blueprint};
/// use pavex_runtime::extract::path::PathParams;
///
/// # fn main() {
/// let mut bp = Blueprint::new();
/// // Register the default constructor and error handler for `PathParams`.
/// bp.constructor(
///     f!(pavex_runtime::extract::path::PathParams::extract),
///     Lifecycle::RequestScoped,
/// ).error_handler(
///     f!(pavex_runtime::extract::path::ExtractPathParamsError::into_response)
/// );
/// // Register a route with a template path segment, `:home_id`.
/// bp.route(GET, "/home/:home_id", f!(crate::get_home));
/// # }
///
/// #[derive(serde::Deserialize)]
/// struct HomePathParams {
///     // The name of the field must match the name of the path parameter
///     // used in `bp.route`.
///     home_id: u32
/// }
///
/// // The `Path` extractor will deserialize the path parameters into the
/// // specified type.
/// fn get_home(params: PathParams<HomePathParams>) -> String {
///    format!("The identifier for this home is: {}", params.0.home_id)
/// }
/// ```
///
/// `home_id` will be set to `1` for an incoming `/home/1` request.  
/// Extraction will fail, instead, if we receive an `/home/abc` request.
///
/// # Supported types
///
/// `T` in `PathParams<T>` must implement [`serde::Deserialize`].  
/// `T` can be one of the following:
///
/// - a struct with named fields, where each field name matches one of the path parameter names
///   used in the route's path template.
/// ```rust
/// # use pavex_builder::{f, router::GET, Blueprint};
/// use pavex_runtime::extract::path::PathParams;
///
/// # fn main() {
/// let mut bp = Blueprint::new();
/// // [...]
/// // Register a route with a few path parameters.
/// bp.route(GET, "/address/:address_id/home/:home_id/room/:room_id/", f!(crate::get_home));
/// # }
///
/// #[derive(serde::Deserialize)]
/// struct HomePathParams {
///     // The name of the field must match the name of the path parameter
///     // used in the template we passed to `bp.route`.
///     home_id: u32,
///     // You can map a path parameter to a struct field with a different
///     // name via the `rename` attribute.
///     #[serde(rename(deserialize = "address_id"))]
///     street_id: String,
///     // You can also choose to ignore some path parameters—e.g. we are not
///     // extracting the `room_id` here.
/// }
///
/// // The `Path` extractor will deserialize the path parameters into the
/// // type you specified—`HomePathParams` in this case.
/// fn get_home(params: PathParams<HomePathParams>) -> String {
///     let params = params.0;
///     format!("The home with id {} is in street {}", params.home_id, params.street_id)
/// }
/// ```
/// - a map-like type, e.g. `HashMap<String, String>`, where the keys are the path parameter names
///   used in the route's path template and the values are the extracted path parameter values.
/// ```rust
/// # use pavex_builder::{f, router::GET, Blueprint};
/// use pavex_runtime::extract::path::PathParams;
/// use std::collections::HashMap;
///
/// # fn main() {
/// let mut bp = Blueprint::new();
/// // [...]
/// // Register a route with a few path parameters.
/// bp.route(GET, "/address/:address_id/home/:home_id/room/:room_id/", f!(crate::get_home));
/// # }
///
/// // The `Path` extractor will collect all the deserialized parameters
/// // as key-value pairs in a `HashMap`.
/// fn get_home(params: PathParams<HashMap<String, String>>) -> String {
///     let params = params.0;
///     format!("The home with id {} is in street {}", params["home_id"], params["street_id"])
/// }
/// ```
///
/// # Unsupported types
///
///
pub struct PathParams<T>(pub T);

impl<T> PathParams<T> {
    /// The default constructor for [`PathParams`].
    ///
    /// If the extraction fails, an [`ExtractPathParamsError`] returned.
    pub fn extract<'key, 'value>(
        params: Params<'key, 'value>,
    ) -> Result<Self, ExtractPathParamsError>
    where
        T: Deserialize<'value>,
        // The parameter ids live as long as the server, while the values are tied to the lifecycle
        // of an incoming request. So it's always true that 'key outlives 'value.
        'key: 'value,
    {
        let mut decoded_params = Vec::with_capacity(params.len());
        for (id, value) in params.iter() {
            let decoded_value = percent_decode_str(value).decode_utf8().map_err(|e| {
                ExtractPathParamsError::InvalidUtf8InPathParameter(InvalidUtf8InPathParam {
                    invalid_key: id.into(),
                    invalid_raw_segment: value.into(),
                    source: e,
                })
            })?;
            decoded_params.push((id, decoded_value));
        }
        let deserializer = PathDeserializer::new(&decoded_params);
        T::deserialize(deserializer)
            .map_err(ExtractPathParamsError::PathDeserializationError)
            .map(PathParams)
    }
}

/// The error returned by [`PathParams::extract`] when the extraction fails.
///
/// See [`PathParams::extract`] and the documentation of each error variant for more details.
///
/// `pavex` provides [`ExtractPathParamsError::into_response`] as the default error handler for
/// this failure.
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
/// URL parameters must be percent-encoded whenever they contain characters that are not
/// URL safe - e.g. whitespaces.
///
/// `pavex` automatically percent-decodes URL parameters before trying to deserialize them
/// in `Path<T>`.
/// This error is returned whenever the percent-decoding step fails - i.e. the decoded data is not a
/// valid UTF8 string.
///
/// # Example
///
/// One of our routes is `/address/:address_id`.
/// We receive a request with `/address/the%20street` as path - `address_id` is set to
/// `the%20street` and `pavex` automatically decodes it into `the street`.
///
/// We could also receive a request using `/address/dirty%DE~%C7%1FY` as path - `address_id`, when
/// decoded, is a sequence of bytes that cannot be interpreted as a well-formed UTF8 string.
/// This error is then returned.
#[error(
    "`{invalid_raw_segment}` cannot be used as `{invalid_key}` \
since it is not a well-formed UTF8 string when percent-decoded"
)]
pub struct InvalidUtf8InPathParam {
    invalid_key: String,
    invalid_raw_segment: String,
    #[source]
    source: Utf8Error,
}

impl ExtractPathParamsError {
    /// Convert an [`ExtractPathParamsError`] into an HTTP response.
    ///
    /// It returns a `500 Internal Server Error` to the caller if the failure was caused by a
    /// programmer error (e.g. `T` in `Path<T>` is an unsupported type).  
    /// It returns a `400 Bad Request` for all other cases.
    pub fn into_response(&self) -> Response<String> {
        match self {
            ExtractPathParamsError::InvalidUtf8InPathParameter(e) => Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(format!("Invalid URL: {}", e))
                .unwrap(),
            ExtractPathParamsError::PathDeserializationError(e) => match e.kind {
                ErrorKind::ParseErrorAtKey { .. }
                | ErrorKind::ParseError { .. }
                | ErrorKind::Message(_) => Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body(format!("Invalid URL: {}", e.kind))
                    .unwrap(),
                ErrorKind::UnsupportedType { .. } => Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body("".to_string())
                    .unwrap(),
            },
        }
    }
}

#[derive(Debug)]
/// Something went wrong when trying to deserialize the percent-decoded URL parameters into
/// the target type you specified—`T` in `Path<T>`.
///
/// You can use [`PathDeserializationError::kind`] to get more details about the error.
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
/// `PathDeserializationError` into an HTTP response).
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
                    "`{name}` is not a supported type for the `Path` extractor. \
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
                "Cannot parse `{key}` with value `{value}` as a `{expected_type}`"
            ),
            ErrorKind::ParseError {
                value,
                expected_type,
            } => write!(f, "Cannot parse `{value}` as a `{expected_type}`"),
        }
    }
}
