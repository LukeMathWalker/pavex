//! px:logger
use pavex::Response;
use pavex::post_process;
use pavex_tracing::{
    RootSpan,
    fields::{HTTP_RESPONSE_STATUS_CODE, http_response_status_code},
};

#[post_process] // px::hl
pub fn response_logger(response: Response, root_span: &RootSpan) -> Response {
    root_span.record(
        HTTP_RESPONSE_STATUS_CODE,
        http_response_status_code(&response),
    );
    response
}
