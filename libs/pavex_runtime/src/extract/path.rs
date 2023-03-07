//! Extract the values of templated path segments from the incoming request using [`Path`].
use matchit::Params;

use crate::response::Response;

/// Extract the values of templated path segments from the incoming request.
///
/// E.g. set `home_id` to `1` when matching `/home/1` against `/home/:home_id`.
pub struct Path<T>(pub T);

impl<T> Path<T> {
    /// The default constructor for [`Path`].
    pub fn extract(_params: Params) -> Result<Self, ExtractPathError> {
        todo!()
    }
}

#[non_exhaustive]
/// The error type for [`Path::extract`].
pub struct ExtractPathError;

impl ExtractPathError {
    /// The default error handler for [`ExtractPathError`].
    ///
    /// It returns a `400 Bad Request` to the caller.
    pub fn default_error_handler(&self) -> Response {
        todo!()
    }
}
