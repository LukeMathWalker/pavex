use std::fmt::Debug;
use std::str::Utf8Error;

use http::StatusCode;
use percent_encoding::percent_decode_str;
use serde::Deserialize;

use crate::extract::route::deserializer::PathDeserializer;
use crate::extract::route::RawRouteParams;
use crate::response::Response;

/// Extract (typed) route parameters from the URL of an incoming request.
///
/// # Example
///
/// ```rust
/// use pavex_builder::{f, router::GET, Blueprint, Lifecycle};
/// use pavex_runtime::extract::route::RouteParams;
///
/// # fn main() {
/// let mut bp = Blueprint::new();
/// // Register the default constructor and error handler for `RouteParams`.
/// bp.constructor(
///     f!(pavex_runtime::extract::path::RouteParams::extract),
///     Lifecycle::RequestScoped,
/// ).error_handler(
///     f!(pavex_runtime::extract::path::ExtractRouteParamsError::into_response)
/// );
/// // Register a route with a route parameter, `:home_id`.
/// bp.route(GET, "/home/:home_id", f!(crate::get_home));
/// # }
///
/// #[derive(serde::Deserialize)]
/// struct HomeRouteParams {
///     // The name of the field must match the name of the route parameter
///     // used in `bp.route`.
///     home_id: u32
/// }
///
/// // The `RouteParams` extractor will deserialize the route parameters into
/// // the type you specified—`HomeRouteParams` in this case.
/// fn get_home(params: &RouteParams<HomeRouteParams>) -> String {
///    format!("The identifier for this home is: {}", params.0.home_id)
/// }
/// ```
///
/// `home_id` will be set to `1` for an incoming `/home/1` request.  
/// Extraction will fail, instead, if we receive an `/home/abc` request.
///
/// # Supported types
///
/// `T` in `RouteParams<T>` must implement [`serde::Deserialize`].  
/// `T` can be one of the following:
///
/// - a struct with named fields, where each field name matches one of the route parameter names
///   used in the route's path template.
/// ```rust
/// use pavex_builder::{f, router::GET, Blueprint};
/// use pavex_runtime::extract::route::RouteParams;
///
/// # fn main() {
/// let mut bp = Blueprint::new();
/// // [...]
/// // Register a route with a few route parameters.
/// bp.route(GET, "/address/:address_id/home/:home_id/room/:room_id/", f!(crate::get_home));
/// # }
///
/// #[derive(serde::Deserialize)]
/// struct HomeRouteParams {
///     // The name of the field must match the name of the route parameter
///     // used in the template we passed to `bp.route`.
///     home_id: u32,
///     // You can map a route parameter to a struct field with a different
///     // name via the `rename` attribute.
///     #[serde(rename(deserialize = "address_id"))]
///     street_id: String,
///     // You can also choose to ignore some route parameters—e.g. we are not
///     // extracting the `room_id` here.
/// }
///
/// // The `RouteParams` extractor will deserialize the route parameters into the
/// // type you specified—`HomeRouteParams` in this case.
/// fn get_home(params: &RouteParams<HomeRouteParams>) -> String {
///     let params = &params.0;
///     format!("The home with id {} is in street {}", params.home_id, params.street_id)
/// }
/// ```
/// - a map-like type, e.g. `HashMap<String, String>`, where the keys are the route parameter names
///   used in the route's path template and the values are the extracted route parameter values.
/// ```rust
/// use pavex_builder::{f, router::GET, Blueprint};
/// use pavex_runtime::extract::route::RouteParams;
/// use std::collections::HashMap;
///
/// # fn main() {
/// let mut bp = Blueprint::new();
/// // [...]
/// // Register a route with a few route parameters.
/// bp.route(GET, "/address/:address_id/home/:home_id/room/:room_id/", f!(crate::get_home));
/// # }
///
/// // All the deserialized (route parameter name, route parameter value) pairs
/// // will be inserted into the map.
/// fn get_home(params: &RouteParams<HashMap<String, u32>>) -> String {
///     let params = &params.0;
///     format!("The home with id {} is in street {}", params["home_id"], params["street_id"])
/// }
/// ```
///
/// # Unsupported types
///
/// `pavex` wants to enable local reasoning, whenever possible: it should be easy to understand what
/// each extracted route parameter represents.  
/// Struct with named fields are ideal in this regard: by looking at the field name you can
/// immediately understand _which_ route parameter is being extracted.  
/// The same is not true for other types, e.g. `(String, u64, u32)`, where you have to go and
/// check the route's path template to understand what each entry represents.
/// For this reason, `pavex` does not support the following types as `T` in `RouteParams<T>`:
///
/// - tuples, e.g. `(u32, String)`;
/// - tuple structs, e.g. `struct HomeId(u32, String)`;
/// - unit structs, e.g. `struct HomeId`;
/// - newtypes, e.g. `struct HomeId(MyParamsStruct)`;
/// - sequence-like types, e.g. `Vec<String>`;
/// - enums.
///
/// # Working with raw route parameters
///
/// It is possible to work with the **raw** route parameters, i.e. the route parameters as they
/// are extracted from the URL, before any kind of percent-decoding or deserialization has taken
/// place.
///
/// You can do so by using the [`RawRouteParams`] extractor instead of [`RouteParams`].
///
/// ```rust
/// use pavex_builder::{f, router::GET, Blueprint};
/// use pavex_runtime::extract::route::RawRouteParams;
///
/// # fn main() {
/// let mut bp = Blueprint::new();
/// // [...]
/// // Register a route with a few route parameters.
/// bp.route(GET, "/address/:address_id/home/:home_id", f!(crate::get_home));
/// # }
///
/// fn get_home(params: &RawRouteParams) -> String {
///     let home_id = &params.get("home_id").unwrap();
///     let street_id = &params.get("street_id").unwrap();
///     format!("The home with id {} is in street {}", home_id, street_id)
/// }
/// ```
///
/// `RawRouteParams` is a built-in extractor, so you don't need to register any constructor for it
/// against the [`Blueprint`] for your application.
pub struct RouteParams<T>(
    /// The extracted route parameters, deserialized into `T`, the type you specified.
    pub T,
);

impl<T> RouteParams<T> {
    /// The default constructor for [`RouteParams`].
    ///
    /// If the extraction fails, an [`ExtractRouteParamsError`] returned.
    pub fn extract<'key, 'value>(
        params: RawRouteParams<'key, 'value>,
    ) -> Result<Self, ExtractRouteParamsError>
    where
        T: Deserialize<'value>,
        // The parameter ids live as long as the server, while the values are tied to the lifecycle
        // of an incoming request. So it's always true that 'key outlives 'value.
        'key: 'value,
    {
        let mut decoded_params = Vec::with_capacity(params.len());
        for (id, value) in params.iter() {
            let decoded_value = percent_decode_str(value).decode_utf8().map_err(|e| {
                ExtractRouteParamsError::InvalidUtf8InPathParameter(InvalidUtf8InPathParam {
                    invalid_key: id.into(),
                    invalid_raw_segment: value.into(),
                    source: e,
                })
            })?;
            decoded_params.push((id, decoded_value));
        }
        let deserializer = PathDeserializer::new(&decoded_params);
        T::deserialize(deserializer)
            .map_err(ExtractRouteParamsError::PathDeserializationError)
            .map(RouteParams)
    }
}

/// The error returned by [`RouteParams::extract`] when the extraction fails.
///
/// See [`RouteParams::extract`] and the documentation of each error variant for more details.
///
/// `pavex` provides [`ExtractRouteParamsError::into_response`] as the default error handler for
/// this failure.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ExtractRouteParamsError {
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

impl ExtractRouteParamsError {
    /// Convert an [`ExtractRouteParamsError`] into an HTTP response.
    ///
    /// It returns a `500 Internal Server Error` to the caller if the failure was caused by a
    /// programmer error (e.g. `T` in `Path<T>` is an unsupported type).  
    /// It returns a `400 Bad Request` for all other cases.
    pub fn into_response(&self) -> Response<String> {
        match self {
            ExtractRouteParamsError::InvalidUtf8InPathParameter(e) => Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(format!("Invalid URL.\n{}", e))
                .unwrap(),
            ExtractRouteParamsError::PathDeserializationError(e) => match e.kind {
                ErrorKind::ParseErrorAtKey { .. } | ErrorKind::ParseError { .. } => {
                    Response::builder()
                        .status(StatusCode::BAD_REQUEST)
                        .body(format!("Invalid URL.\n{}", e.kind))
                        .unwrap()
                }
                // We put the "custom" message variant here as well because it's not clear
                // whether it's a programmer error or not. We err on the side of safety and
                // prefer to return a 500 with an opaque error message.
                ErrorKind::Message(_) | ErrorKind::UnsupportedType { .. } => Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body("Something went wrong when trying to process the request".to_string())
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

/// The kinds of errors that can happen when deserializing into a [`RouteParams`].
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
                    "`{name}` is not a supported type for the `RouteParams` extractor. \
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
