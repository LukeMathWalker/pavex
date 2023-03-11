//! Extract the values of templated path segments from the incoming request using [`Path`].
use std::str::Utf8Error;

use matchit::Params;
use percent_encoding::percent_decode_str;

use crate::response::Response;

/// Extract templated path segments from the incoming request.
///
/// E.g. set `home_id` to `1` when matching `/home/1` against `/home/:home_id`.
pub struct Path<T>(pub T);

impl<T> Path<T> {
    /// The default constructor for [`Path`].
    pub fn extract(params: Params) -> Result<Self, ExtractPathError> {
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
        todo!()
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
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
/// URL parameters must be percent-encoded whenever they contain characters that are not
/// URL safe - e.g. whitespaces.
///
/// `pavex` automatically percent-decodes URL parameters before trying to deserialize them
/// in `Path<T>.
/// This error is returned whenever the decoding step fails - i.e. the decoded data is not a
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
