use crate::RootSpan;
use pavex::middleware::Next;
use pavex::response::Response;
use std::future::IntoFuture;
use tracing::Instrument;

/// A logging middleware that instruments the request processing pipeline with
/// [`RootSpan`].
/// All `tracing` spans entered after `logger` executes will be children of [`RootSpan`],
/// either directly or transitively.
///
/// # Registration
///
/// Use [`Blueprint::wrap`] to register `logger` as a middleware:
///
/// ```rust
/// use pavex::blueprint::Blueprint;
/// use pavex_tracing::LOGGER;
///
/// let mut bp = Blueprint::new();
/// bp.wrap(LOGGER);
/// ```
///
/// You will also need to register a constructor for [`RootSpan`].
/// Check out its documentation for more information.
///
/// [`Blueprint::wrap`]: pavex::blueprint::Blueprint::wrap
#[pavex::wrap]
pub async fn logger<C>(root_span: RootSpan, next: Next<C>) -> Response
where
    C: IntoFuture<Output = Response>,
{
    next.into_future().instrument(root_span.into_inner()).await
}
