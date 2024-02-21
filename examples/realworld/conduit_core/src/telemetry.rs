use pavex::http::Version;
use pavex::middleware::Next;
use pavex::request::path::MatchedPathPattern;
use pavex::request::RequestHead;
use pavex::response::Response;
use pavex::telemetry::ServerRequestId;
use pavex_tracing::RootSpan;
use std::borrow::Cow;
use std::future::IntoFuture;
use tokio::task::JoinHandle;

pub fn root_span(
    request_head: &RequestHead,
    matched_route: MatchedPathPattern,
    server_request_id: ServerRequestId,
) -> RootSpan {
    let user_agent = request_head
        .headers
        .get("User-Agent")
        .map(|h| h.to_str().unwrap_or_default())
        .unwrap_or_default();

    let span = tracing::info_span!(
        "HTTP request",
        request_id = %server_request_id,
        http.method = %request_head.method,
        http.flavor = %http_flavor(request_head.version),
        user_agent.original = %user_agent,
        http.response.status_code = tracing::field::Empty,
        http.route = %matched_route,
        http.target = %request_head.target.path_and_query().map(|p| p.as_str()).unwrap_or(""),
        // 👇 fields that we can't fill out _yet_ because we don't have access to connection info
        //
        // http.scheme = %$crate::root_span_macro::private::http_scheme(connection_info.scheme()),
        // http.host = %connection_info.host(),
        // http.client_ip = %$request.connection_info().realip_remote_addr().unwrap_or(""),
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

pub async fn response_logger<T>(next: Next<T>, root_span: &RootSpan) -> Response
where
    T: IntoFuture<Output = Response>,
{
    let response = next.await;
    root_span.record("http.response.status_code", &response.status().as_u16());
    response
}

/// Spawn a blocking task without losing the current `tracing` span.
///
/// # Why is this needed?
///
/// `tracing`'s span context is thread-local, so when a blocking task is spawned
/// the current span is lost. This function spawns a blocking task and
/// explicitly re-attaches the current span to the workload in
/// the new thread.
pub fn spawn_blocking_with_tracing<F, R>(f: F) -> JoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let current_span = tracing::Span::current();
    tokio::task::spawn_blocking(move || current_span.in_scope(f))
}
