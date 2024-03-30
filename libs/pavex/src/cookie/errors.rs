//! Errors that can occur when working with cookies.
use crate::response::Response;
pub use biscotti::errors::*;
use http::header::ToStrError;

#[derive(Debug, thiserror::Error)]
/// The error type returned by [`extract_request_cookies`](super::extract_request_cookies).
pub enum ExtractRequestCookiesError {
    #[error("Some characters in the `Cookie` header aren't printable ASCII characters.")]
    InvalidHeaderValue(#[from] ToStrError),
    #[error("Failed to parse request cookies out of the `Cookie` header.")]
    ParseError(#[from] ParseError),
}

impl ExtractRequestCookiesError {
    /// Convert an [`ExtractRequestCookiesError`] into an HTTP response.
    ///
    /// It returns a `400 Bad Request` to the caller.
    /// The body provides details on what exactly went wrong.
    pub fn into_response(&self) -> Response {
        use std::fmt::Write as _;

        let mut body = self.to_string();
        match self {
            ExtractRequestCookiesError::InvalidHeaderValue(_) => {}
            ExtractRequestCookiesError::ParseError(e) => {
                let _ = write!(&mut body, "\n{e}");
            }
        }
        Response::bad_request().set_typed_body(body)
    }
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
#[error("Some characters in the `Set-Cookie` header value are not printable ASCII characters.")]
/// The error type returned by [`inject_response_cookies`](super::inject_response_cookies).
pub struct InjectResponseCookiesError {
    /// The invalid header value.
    pub invalid_header_value: String,
}

impl InjectResponseCookiesError {
    /// Convert an [`InjectResponseCookiesError`] into an HTTP response.
    ///
    /// It returns a `500 Internal Server Error` to the caller,
    /// since failure is likely due to misconfiguration or
    /// mismanagement on the server side.
    pub fn into_response(&self) -> Response {
        Response::internal_server_error()
    }
}
