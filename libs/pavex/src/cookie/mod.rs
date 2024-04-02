//! Everything you need to work with HTTP cookies.
//!
//! Most types and functions are re-exports of the
//! [`biscotti@0.3`](https://docs.rs/biscotti/0.3) crate.
use crate::cookie::errors::{ExtractRequestCookiesError, InjectResponseCookiesError};
use crate::request::RequestHead;
use crate::response::Response;
// Everything from `biscotti`, except the `time` module and `ResponseCookies`.
pub use biscotti::{
    config, request, Expiration, Key, Processor, RemovalCookie, RequestCookie, RequestCookies,
    ResponseCookie, ResponseCookieId, SameSite,
};
pub mod errors;
pub mod response;
use http::header::{COOKIE, SET_COOKIE};
use http::HeaderValue;

mod kit;
mod response_cookies;
use crate::error::UnexpectedError;
pub use kit::CookieKit;
pub use response_cookies::ResponseCookies;

/// Parse cookies out of the incoming request.
///
/// It's the default constructor for [`RequestCookies`].
pub fn extract_request_cookies<'request, 'b>(
    request_head: &'request RequestHead,
    processor: &'b Processor,
) -> Result<RequestCookies<'request>, ExtractRequestCookiesError> {
    let mut cookies = RequestCookies::new();
    for header in request_head.headers.get_all(COOKIE).into_iter() {
        let header = header
            .to_str()
            .map_err(|e| ExtractRequestCookiesError::InvalidHeaderValue(e))?;
        cookies.extend_from_header(header, processor).map_err(|e| {
            use biscotti::errors::ParseError::*;
            match e {
                MissingPair(e) => ExtractRequestCookiesError::MissingPair(e),
                EmptyName(e) => ExtractRequestCookiesError::EmptyName(e),
                Crypto(e) => ExtractRequestCookiesError::Crypto(e),
                Decoding(e) => ExtractRequestCookiesError::Decoding(e),
                _ => ExtractRequestCookiesError::Unexpected(UnexpectedError::new(e)),
            }
        })?;
    }
    Ok(cookies)
}

/// Attach cookies to the outgoing response.
///
/// It consumes [`ResponseCookies`] by value since no response cookies should be
/// added after the execution of this middleware.
pub fn inject_response_cookies(
    mut response: Response,
    response_cookies: ResponseCookies,
    processor: &Processor,
) -> Result<Response, InjectResponseCookiesError> {
    for value in response_cookies.header_values(processor) {
        let value = HeaderValue::from_str(&value).map_err(|_| InjectResponseCookiesError {
            invalid_header_value: value,
        })?;
        response = response.append_header(SET_COOKIE, value);
    }
    Ok(response)
}
