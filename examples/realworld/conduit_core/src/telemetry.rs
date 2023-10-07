use pavex::middleware::Next;
use pavex::response::Response;
use std::future::IntoFuture;
use tokio::task::JoinHandle;

pub async fn logger<T>(next: Next<T>) -> Response
where
    T: IntoFuture<Output = Response>,
{
    next.await
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
