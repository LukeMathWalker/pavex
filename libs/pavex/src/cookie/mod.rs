//! Everything you need to work with HTTP cookies.
//!
//! Most types and functions are re-exports of the
//! [`biscotti@0.2`](https://docs.rs/biscotti/0.2) crate.
use crate::cookie::errors::InjectResponseCookiesError;
use crate::request::RequestHead;
use crate::response::Response;
// Everything from `biscotti`, except the `time` module
pub use biscotti::{
    config, request, Expiration, Key, Processor, RemovalCookie, RequestCookie, RequestCookies,
    ResponseCookie, ResponseCookieId, ResponseCookies, SameSite,
};
use http::header::{COOKIE, SET_COOKIE};
use http::HeaderValue;

mod kit;
pub use kit::CookieKit;

/// Parse cookies out of the incoming request.
///
/// It's the default constructor for [`RequestCookies`].
pub fn extract_request_cookies<'a, 'b>(
    request_head: &'a RequestHead,
    processor: &'b Processor,
) -> Result<RequestCookies<'a>, errors::ExtractRequestCookiesError> {
    // TODO: Avoid allocation once `biscotti`'s API allows it.
    let cookie_headers = request_head
        .headers
        .get_all(COOKIE)
        .into_iter()
        .map(|h| h.to_str())
        .collect::<Result<Vec<_>, _>>()?;
    let cookies = RequestCookies::parse_headers(cookie_headers.into_iter(), processor)?;
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
        let value = HeaderValue::from_str(&value)
            .map_err(|_| InjectResponseCookiesError {
                invalid_header_value: value
            })?;
        response = response.append_header(SET_COOKIE, value);
    }
    Ok(response)
}

pub mod errors;

