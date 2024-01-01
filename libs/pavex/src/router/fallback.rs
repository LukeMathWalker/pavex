use crate::http::header::ALLOW;
use crate::response::Response;

use super::AllowedMethods;

/// The default fallback handler for incoming requests that don't match
/// any of the routes you registered.
///
/// It returns a `404 Not Found` response if the path doesn't match any of the
/// registered route paths.  
/// It returns a `405 Method Not Allowed` response if the path matches a
/// registered route path but the method doesn't match any of its associated
/// handlers.
///
/// It also returns a `404 Not Found` response if the path allows all HTTP methods,
/// including custom ones.
pub async fn default_fallback(allowed_methods: &AllowedMethods) -> Response {
    if let Some(header_value) = allowed_methods.allow_header_value() {
        Response::method_not_allowed().insert_header(ALLOW, header_value)
    } else {
        Response::not_found()
    }
}
