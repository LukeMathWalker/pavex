use pavex::{response::Response};
use pavex_tracing::{
    RootSpan,
    fields::{http_response_status_code, HTTP_RESPONSE_STATUS_CODE}
};

pub fn response_logger(response: Response, root_span: &RootSpan) -> Response
{
    root_span.record(
        HTTP_RESPONSE_STATUS_CODE,
        http_response_status_code(&response),
    );
    response
}