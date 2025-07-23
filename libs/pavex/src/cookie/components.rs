use crate::cookie::ResponseCookies;
use crate::cookie::errors::{ExtractRequestCookiesError, InjectResponseCookiesError};
use crate::error::UnexpectedError;
use crate::request::RequestHead;
use crate::response::Response;
use biscotti::{Processor, RequestCookies};
use http::HeaderValue;
use http::header::{COOKIE, SET_COOKIE};
use pavex_macros::{post_process, request_scoped};

/// Parse cookies out of the incoming request.
///
/// It's the default constructor for [`RequestCookies`].
#[request_scoped(pavex = crate)]
pub fn extract_request_cookies<'request>(
    request_head: &'request RequestHead,
    processor: &Processor,
) -> Result<RequestCookies<'request>, ExtractRequestCookiesError> {
    let mut cookies = RequestCookies::new();
    for header in request_head.headers.get_all(COOKIE).into_iter() {
        let header = header
            .to_str()
            .map_err(ExtractRequestCookiesError::InvalidHeaderValue)?;
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
#[post_process(pavex = crate)]
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
