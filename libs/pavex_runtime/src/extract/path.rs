//! Extract the values of templated path segments from the incoming request.
use matchit::Params;

use crate::response::Response;

/// Extract the values of templated path segments from the incoming request.
///
/// E.g. `home_id=1` in `/home/:home_id` from `/home/1`.
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
