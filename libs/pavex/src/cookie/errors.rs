//! Errors that can occur when working with cookies.
use crate::Response;
use crate::error::UnexpectedError;
pub use biscotti::errors::*;
use http::header::ToStrError;
use pavex_macros::methods;

#[derive(Debug)]
#[non_exhaustive]
/// The error type returned by [`extract_request_cookies`](super::extract_request_cookies).
pub enum ExtractRequestCookiesError {
    InvalidHeaderValue(ToStrError),
    MissingPair(MissingPairError),
    EmptyName(EmptyNameError),
    Crypto(CryptoError),
    Decoding(DecodingError),
    Unexpected(UnexpectedError),
}

impl std::fmt::Display for ExtractRequestCookiesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExtractRequestCookiesError::InvalidHeaderValue(_) => {
                write!(
                    f,
                    "Some characters in the `Cookie` header aren't printable ASCII characters"
                )
            }
            _ => {
                write!(
                    f,
                    "Failed to parse request cookies out of the `Cookie` header"
                )
            }
        }
    }
}

impl std::error::Error for ExtractRequestCookiesError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ExtractRequestCookiesError::InvalidHeaderValue(e) => Some(e),
            ExtractRequestCookiesError::MissingPair(e) => Some(e),
            ExtractRequestCookiesError::EmptyName(e) => Some(e),
            ExtractRequestCookiesError::Crypto(e) => Some(e),
            ExtractRequestCookiesError::Decoding(e) => Some(e),
            ExtractRequestCookiesError::Unexpected(e) => Some(e),
        }
    }
}

#[methods]
impl ExtractRequestCookiesError {
    /// Convert an [`ExtractRequestCookiesError`] into an HTTP response.
    ///
    /// It returns a `400 Bad Request` to the caller.
    /// The body provides details on what exactly went wrong.
    #[error_handler(pavex = crate)]
    pub fn into_response(&self) -> Response {
        use std::fmt::Write as _;

        let mut body = self.to_string();
        match self {
            ExtractRequestCookiesError::MissingPair(e) => {
                write!(body, ". {e}").ok();
            }
            ExtractRequestCookiesError::EmptyName(e) => {
                write!(body, ". {e}").ok();
            }
            ExtractRequestCookiesError::Decoding(e) => {
                write!(body, ". {e}").ok();
            }
            ExtractRequestCookiesError::Unexpected(_)
            | ExtractRequestCookiesError::InvalidHeaderValue(_)
            | ExtractRequestCookiesError::Crypto(_) => {}
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

#[methods]
impl InjectResponseCookiesError {
    /// Convert an [`InjectResponseCookiesError`] into an HTTP response.
    ///
    /// It returns a `500 Internal Server Error` to the caller,
    /// since failure is likely due to misconfiguration or
    /// mismanagement on the server side.
    #[error_handler(pavex = crate)]
    pub fn into_response(&self) -> Response {
        Response::internal_server_error()
    }
}
