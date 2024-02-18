use pavex::http::Version;
use pavex::middleware::Next;
use pavex::request::path::MatchedPathPattern;
use pavex::request::RequestHead;
use pavex::response::Response;
use pavex::telemetry::ServerRequestId;
use pavex_tracing::RootSpan;
use std::borrow::Cow;
use std::future::IntoFuture;
use tracing::Instrument;

/// A logging middleware that wraps the request pipeline in the root span.
/// It takes care to record key information about the request and the response.
pub async fn logger<T>(next: Next<T>, root_span: RootSpan) -> Response
where
    T: IntoFuture<Output = Response>,
{
    let response = next
        .into_future()
        .instrument(root_span.clone().into_inner())
        .await;
    root_span.record("http.response.status_code", &response.status().as_u16());
    response
}

/// An error observer to log error details.
///
/// It emits an error event and attaches information about the error to the root span.
/// If multiple errors are observed for the same request, it will emit multiple error events
/// but only the details of the last error will be attached to the root span.
pub async fn log_error(e: &pavex::Error, root_span: RootSpan) {
    let source_chain = error_source_chain(e);
    tracing::error!(
        error.msg = %e,
        error.details = ?e,
        error.source_chain = %source_chain,
        "An error occurred during request handling",
    );
    root_span.record("error.msg", tracing::field::display(e));
    root_span.record("error.details", tracing::field::debug(e));
    root_span.record("error.source_chain", error_source_chain(e));
}

fn error_source_chain(e: &pavex::Error) -> String {
    use std::error::Error as _;
    use std::fmt::Write as _;

    let mut chain = String::new();
    let mut source = e.source();
    while let Some(s) = source {
        let _ = writeln!(chain, "- {}", s);
        source = s.source();
    }
    chain
}

/// Construct a new root span for the given request.
///
/// We follow OpenTelemetry's HTTP semantic conventions as closely as
/// possible for field naming.
pub fn root_span(
    request_head: &RequestHead,
    matched_route: MatchedPathPattern,
    request_id: &ServerRequestId,
) -> RootSpan {
    let user_agent = request_head
        .headers
        .get("User-Agent")
        .map(|h| h.to_str().unwrap_or_default())
        .unwrap_or_default();

    let span = tracing::info_span!(
        "HTTP request",
        http.method = %request_head.method,
        http.flavor = %http_flavor(request_head.version),
        user_agent.original = %user_agent,
        http.response.status_code = tracing::field::Empty,
        http.route = %matched_route,
        http.target = %request_head.target.path_and_query().map(|p| p.as_str()).unwrap_or(""),
        request_id = %request_id,
        error.msg = tracing::field::Empty,
        error.details = tracing::field::Empty,
        error.source_chain = tracing::field::Empty,
    );
    RootSpan::new(span)
}

fn http_flavor(version: Version) -> Cow<'static, str> {
    match version {
        Version::HTTP_09 => "0.9".into(),
        Version::HTTP_10 => "1.0".into(),
        Version::HTTP_11 => "1.1".into(),
        Version::HTTP_2 => "2.0".into(),
        Version::HTTP_3 => "3.0".into(),
        other => format!("{other:?}").into(),
    }
}
