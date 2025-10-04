use pavex::Blueprint;
use pavex::Response;
use pavex::request::RequestHead;
use pavex::request::path::MatchedPathPattern;
use pavex::telemetry::ServerRequestId;
use pavex::{error_observer, post_process, request_scoped};
use pavex_tracing::fields::{
    ERROR_DETAILS, ERROR_MESSAGE, ERROR_SOURCE_CHAIN, HTTP_REQUEST_METHOD, HTTP_REQUEST_SERVER_ID,
    HTTP_RESPONSE_STATUS_CODE, HTTP_ROUTE, NETWORK_PROTOCOL_VERSION, URL_PATH, URL_QUERY,
    USER_AGENT_ORIGINAL, error_details, error_message, error_source_chain, http_request_method,
    http_request_server_id, http_response_status_code, http_route, network_protocol_version,
    url_path, url_query, user_agent_original,
};
use pavex_tracing::{LOGGER, RootSpan};
use tracing_log_error::log_error;

/// Register telemetry middlewares and an error observer with the application blueprint.
pub(crate) fn instrument(bp: &mut Blueprint) {
    bp.wrap(LOGGER);
    bp.post_process(RESPONSE_LOGGER);
    bp.error_observer(ERROR_LOGGER);
}

/// Construct a new root span for the given request.
#[request_scoped(clone_if_necessary)]
pub fn root_span(
    request_head: &RequestHead,
    matched_path_pattern: MatchedPathPattern,
    request_id: ServerRequestId,
) -> RootSpan {
    // We use the `{ <expr> }` syntax to tell `tracing` that it should
    // interpret those identifiers as expressions rather than string literals.
    // I.e. `{ HTTP_REQUEST_METHOD }` should be evaluated as "http.request.method"
    // rather than "HTTP_REQUEST_METHOD".
    let span = tracing::info_span!(
        "HTTP request",
        { HTTP_REQUEST_METHOD } = http_request_method(request_head),
        { HTTP_REQUEST_SERVER_ID } = http_request_server_id(request_id),
        { HTTP_ROUTE } = http_route(matched_path_pattern),
        { NETWORK_PROTOCOL_VERSION } = network_protocol_version(request_head),
        { URL_QUERY } = url_query(request_head),
        { URL_PATH } = url_path(request_head),
        { USER_AGENT_ORIGINAL } = user_agent_original(request_head),
        // These fields will be populated later by
        // `response_logger` and (if necessary) by `error_logger`.
        // Nonetheless, they must be declared upfront since `tracing`
        // requires all fields on a span to be known when the span
        // is created, even if they don't have a value (yet).
        { HTTP_RESPONSE_STATUS_CODE } = tracing::field::Empty,
        { ERROR_MESSAGE } = tracing::field::Empty,
        { ERROR_DETAILS } = tracing::field::Empty,
        { ERROR_SOURCE_CHAIN } = tracing::field::Empty,
    );
    RootSpan::new(span)
}

/// Enrich [`RootSpan`] with information extracted from the outgoing response.
#[post_process]
pub async fn response_logger(response: Response, root_span: &RootSpan) -> Response {
    root_span.record(
        HTTP_RESPONSE_STATUS_CODE,
        http_response_status_code(&response),
    );
    response
}

/// An error observer to log error details.
///
/// It emits an error event and attaches information about the error to the root span.
/// If multiple errors are observed for the same request, it will emit multiple error events
/// but only the details of the last error will be attached to the root span.
#[error_observer]
pub async fn error_logger(e: &pavex::Error, root_span: &RootSpan) {
    log_error!(e, "An error occurred during request handling");
    root_span.record(ERROR_MESSAGE, error_message(e));
    root_span.record(ERROR_DETAILS, error_details(e));
    root_span.record(ERROR_SOURCE_CHAIN, error_source_chain(e));
}
