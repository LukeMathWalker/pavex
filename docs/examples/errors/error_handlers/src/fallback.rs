//! px:custom_fallback
use pavex::{
    Response, error_handler,
    http::{HeaderValue, header::LOCATION},
};

#[error_handler]
/// If there is no specific error handler for the given error,
/// redirect to a generic error page.
pub fn redirect_to_error_page(_e: &pavex::Error) -> Response {
    let destination = HeaderValue::from_static("error");
    Response::temporary_redirect().insert_header(LOCATION, destination)
}
