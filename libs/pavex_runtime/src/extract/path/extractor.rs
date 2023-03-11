use std::fmt::Debug;
use std::str::Utf8Error;

use matchit::Params;
use percent_encoding::percent_decode_str;
use serde::Deserialize;

use crate::extract::path::deserializer::PathDeserializer;
use crate::response::Response;

/// Extract templated path segments from the incoming request.
///
/// E.g. set `home_id` to `1` when matching `/home/1` against `/home/:home_id`.
pub struct Path<T>(pub T);

impl<T> Path<T> {
    /// The default constructor for [`Path`].
    pub fn extract<'key, 'value>(params: Params<'key, 'value>) -> Result<Self, ExtractPathError>
    where
        T: Deserialize<'value>,
        // The parameter ids live as long as the server, while the values are tied to the lifecycle
        // of an incoming request. So it's always true that 'key outlives 'value.
        'key: 'value,
    {
        let mut decoded_params = Vec::with_capacity(params.len());
        for (id, value) in params.iter() {
            let decoded_value = percent_decode_str(value).decode_utf8().map_err(|e| {
                ExtractPathError::InvalidUtf8InPathParameter(InvalidUtf8InPathParameter {
                    invalid_key: id.into(),
                    invalid_raw_segment: value.into(),
                    source: e,
                })
            })?;
            decoded_params.push((id, decoded_value));
        }
        let deserializer = PathDeserializer::new(&decoded_params);
        T::deserialize(deserializer)
            .map_err(ExtractPathError::PathDeserializationError)
            .map(Path)
    }
}

/// The error returned by [`Path::extract`] when the extraction fails.
///
/// See [`Path::extract`] and the documentation of each error variant for more details.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ExtractPathError {
    #[error(transparent)]
    /// See [`InvalidUtf8InPathParameter`] for details.
    InvalidUtf8InPathParameter(InvalidUtf8InPathParameter),
    #[error(transparent)]
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
pub struct InvalidUtf8InPathParameter {
    invalid_key: String,
    invalid_raw_segment: String,
    #[source]
    source: Utf8Error,
}

impl ExtractPathError {
    /// The default error handler for [`ExtractPathError`].
    ///
    /// It returns a `400 Bad Request` to the caller.
    pub fn default_error_handler(&self) -> Response {
        todo!()
    }
}

#[derive(Debug)]
pub struct PathDeserializationError {
    pub(super) kind: ErrorKind,
}

impl PathDeserializationError {
    pub(super) fn new(kind: ErrorKind) -> Self {
        Self { kind }
    }

    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }

    pub(super) fn wrong_number_of_parameters() -> WrongNumberOfParameters<()> {
        WrongNumberOfParameters { got: () }
    }

    #[track_caller]
    pub(super) fn unsupported_type(name: &'static str) -> Self {
        Self::new(ErrorKind::UnsupportedType { name })
    }
}

pub(super) struct WrongNumberOfParameters<G> {
    got: G,
}

impl<G> WrongNumberOfParameters<G> {
    #[allow(clippy::unused_self)]
    pub(super) fn got<G2>(self, got: G2) -> WrongNumberOfParameters<G2> {
        WrongNumberOfParameters { got }
    }
}

impl WrongNumberOfParameters<usize> {
    pub(super) fn expected(self, expected: usize) -> PathDeserializationError {
        PathDeserializationError::new(ErrorKind::WrongNumberOfParameters {
            got: self.got,
            expected,
        })
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

/// The kinds of errors that can happen we deserializing into a [`Path`].
///
/// This type is obtained through [`PathDeserializationError::kind`] and is useful for building
/// more precise error messages.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ErrorKind {
    /// The URI contained the wrong number of parameters.
    WrongNumberOfParameters {
        /// The number of actual parameters in the URI.
        got: usize,
        /// The number of expected parameters.
        expected: usize,
    },

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

    /// Failed to parse the value at a specific index into the expected type.
    ///
    /// This variant is used when deserializing into sequence types, such as tuples.
    ParseErrorAtIndex {
        /// The index at which the value was located.
        index: usize,
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

    /// Tried to serialize into an unsupported type such as nested maps.
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
            ErrorKind::WrongNumberOfParameters { got, expected } => {
                write!(
                    f,
                    "Wrong number of path arguments for `Path`. Expected {expected} but got {got}"
                )?;

                if *expected == 1 {
                    write!(f, ". Note that multiple parameters must be extracted with a tuple `Path<(_, _)>` or a struct `Path<YourParams>`")?;
                }

                Ok(())
            }
            ErrorKind::UnsupportedType { name } => write!(f, "Unsupported type `{name}`"),
            ErrorKind::ParseErrorAtKey {
                key,
                value,
                expected_type,
            } => write!(
                f,
                "Cannot parse `{key}` with value `{value:?}` to a `{expected_type}`"
            ),
            ErrorKind::ParseError {
                value,
                expected_type,
            } => write!(f, "Cannot parse `{value:?}` to a `{expected_type}`"),
            ErrorKind::ParseErrorAtIndex {
                index,
                value,
                expected_type,
            } => write!(
                f,
                "Cannot parse value at index {index} with value `{value:?}` to a `{expected_type}`"
            ),
        }
    }
}
