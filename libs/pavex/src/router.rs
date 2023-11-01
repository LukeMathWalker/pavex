//! Dispatch requests to the appropriate handler.

use bytes::Bytes;
use http_body_util::Empty;

use crate::response::Response;

/// The default fallback handler for incoming requests that don't match
/// any of the routes you registered.
///
/// It returns a `404 Not Found` response if the path doesn't match any of the
/// registered route paths.  
/// It returns a `405 Method Not Allowed` response if the path matches a
/// registered route path but the method doesn't match any of its associated
/// handlers.
pub async fn default_fallback() -> Response<Empty<Bytes>> {
    Response::not_found()
}
