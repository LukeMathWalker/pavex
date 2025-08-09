use crate::Response;
use crate::cookie::ResponseCookies;
use crate::cookie::errors::{ExtractRequestCookiesError, InjectResponseCookiesError};
use crate::error::UnexpectedError;
use crate::request::RequestHead;
use biscotti::{Processor, RequestCookies};
use http::HeaderValue;
use http::header::{COOKIE, SET_COOKIE};
use pavex_macros::{post_process, request_scoped};
use tracing_log_error::log_error;

/// Parse cookies out of the incoming request.
///
/// It's the default constructor for [`RequestCookies`].
#[request_scoped(pavex = crate)]
pub fn extract_request_cookies<'request>(
    request_head: &'request RequestHead,
    processor: &Processor,
) -> Result<RequestCookies<'request>, ExtractRequestCookiesError> {
    fn extract_request_cookie<'request>(
        header: &'request HeaderValue,
        processor: &Processor,
        cookies: &mut RequestCookies<'request>,
    ) -> Result<(), ExtractRequestCookiesError> {
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
        Ok(())
    }

    let mut cookies = RequestCookies::new();
    for header in request_head.headers.get_all(COOKIE).into_iter() {
        // Per RFC 6265 (HTTP State Management Mechanism), servers are free to ignore the Cookie
        // header entirely (Section 4.2.2), and the spec places no requirement to reject requests
        // containing malformed cookies. In practice, major frameworks (Tomcat, Django, Express, etc.)
        // use tolerant, best-effort parsing to avoid breaking legitimate requests just because one
        // cookie is corrupt, truncated, or non-compliant.
        //
        // We follow the same approach: skip only the invalid cookie(s) and keep the rest.
        // This prevents unnecessary 4xx/5xx responses, avoids breaking user sessions due to
        // transient client or proxy bugs, and mitigates the risk of denial-of-service from
        // intentionally malformed Cookie headers.
        if let Err(e) = extract_request_cookie(header, processor, &mut cookies) {
            log_error!(
                e,
                level: tracing::Level::WARN,
                "A request cookie is invalid, ignoring it"
            );
        }
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
