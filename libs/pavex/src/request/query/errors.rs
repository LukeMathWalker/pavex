//! Errors that can happen when extracting query parameters.

use pavex_macros::methods;

use crate::Response;

/// The error returned by [`QueryParams::extract`] when the extraction fails.
///
/// See [`QueryParams::extract`] and the documentation of each error variant for more details.
///
/// Pavex provides [`ExtractQueryParamsError::into_response`] as the default error handler for
/// this failure.
///
/// [`QueryParams::extract`]: crate::request::query::QueryParams::extract
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ExtractQueryParamsError {
    #[error(transparent)]
    /// See [`QueryDeserializationError`] for details.
    QueryDeserializationError(QueryDeserializationError),
}

#[methods]
impl ExtractQueryParamsError {
    /// Convert an [`ExtractQueryParamsError`] into an HTTP response.
    ///
    /// It returns a `400 Bad Request` to the caller.
    #[error_handler(pavex = crate)]
    pub fn into_response(&self) -> Response {
        match self {
            Self::QueryDeserializationError(e) => {
                Response::bad_request().set_typed_body(format!("Invalid query parameters.\n{e:?}"))
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
/// Something went wrong when trying to deserialize the percent-decoded query parameters into
/// the target type you specified—`T` in [`QueryParams<T>`].
///
/// [`QueryParams<T>`]: crate::request::query::QueryParams
pub struct QueryDeserializationError {
    inner: serde_path_to_error::Error<serde_html_form::de::Error>,
}

impl QueryDeserializationError {
    pub(super) fn new(e: serde_path_to_error::Error<serde_html_form::de::Error>) -> Self {
        Self { inner: e }
    }
}
