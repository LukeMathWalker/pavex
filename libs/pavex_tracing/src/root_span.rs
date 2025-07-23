/// `RootSpan` is the top-level *logical* [`tracing::Span`] for an incoming request.
///
/// It is not necessarily the top-level *physical* span, as it may be a child of
/// another span (e.g. a span representing the underlying HTTP connection).
///
/// # What's the purpose of a root span?
///
/// The root span should contain enough information, on its own, to determine
/// what happened to a request.
/// It is good practice to enrich the root span throughout the request processing lifecycle:
///
/// - With data from the incoming request, when it's created (e.g. method, path, etc.)
/// - With data that's parsed from the incoming request at a later stage (e.g. user id, if authenticated)
/// - With data from the processing (e.g. error information, if processing failed)
/// - With data from the response (e.g. status code)
///
/// You can read [Stripe's "Canonical log line" blog post](https://stripe.com/blog/canonical-log-lines)
/// for more details on the benefits of this pattern.
///
/// # Accessing `RootSpan`
///
/// You can leverage [Pavex's dependency injection system](https://pavex.dev/docs/guide/dependency_injection/)
/// to access the root span from any of your components—request handlers,
/// middlewares, error observers, etc.
/// It's enough to add an input parameter with type `&RootSpan` to your component.
///
/// ```rust
/// use pavex::Blueprint;
/// use pavex::middleware::Next;
/// use pavex::response::Response;
/// use pavex_tracing::fields::{http_response_status_code, HTTP_RESPONSE_STATUS_CODE};
/// use pavex_tracing::RootSpan;
/// use std::future::IntoFuture;
///
/// /// A middleware to enrich `RootSpan` with information extracted from the
/// /// outgoing response.
/// pub async fn response_logger<T>(next: Next<T>, root_span: &RootSpan) -> Response
///     where
///         T: IntoFuture<Output = Response>,
/// {
///     let response = next.await;
///     root_span.record(
///         HTTP_RESPONSE_STATUS_CODE,
///         http_response_status_code(&response),
///     );
///     response
/// }
/// ```
///
/// # Why is there no default constructor for `RootSpan` in `pavex_tracing`?
///
/// `pavex_tracing` defines `RootSpan` but, unlike other first-party extractors in Pavex,
/// it doesn't provide a default constructor.
/// It's an intentional choice; it stems from the way the `tracing` crate works:
/// every field on a `Span` must be declared when the `Span` is created.
/// **You can't add an extra field after span creation.**
///
/// ```rust
/// use tracing::info_span;
///
/// let span = info_span!("My span");
/// // This won't work!
/// // `span` didn't define `custom_field` as one of its fields when it was created,
/// // so `tracing` ignores our attempt to set it.
/// span.record("custom_field", "field_value");
///
/// let span = info_span!("My span", custom_field = tracing::field::Empty);
/// // This works: we declared `custom_field` when `span` was created,
/// // even though we didn't assign a value to it.
/// // `tracing` honors our intention here and sets `custom_field` to `field_value`.
/// span.record("custom_field", "field_value");
/// ```
///
/// Over time, every application wants to enrich its [`RootSpan`] with domain-specific fields or
/// have tighter control over the way "default" fields are named or populated.
/// To make it happen, you need to control span creation, which in turn implies that you need
/// to control the constructor.
/// For this very reason, `pavex new` scaffolds a constructor for [`RootSpan`] **in your own project**,
/// which you are then free to customize and tailor to your own needs when the time comes.
///
/// You can leverage the helpers defined in the [`fields`](crate::fields) module to keep
/// your field names (and the way their values are represented) in line with a "standard"
/// Pavex application.
///
/// [`tracing::Span`]: https://docs.rs/tracing/0.1.40/tracing/struct.Span.html
#[derive(Debug, Clone)]
pub struct RootSpan(tracing::Span);

impl std::ops::Deref for RootSpan {
    type Target = tracing::Span;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl RootSpan {
    /// Create a new [`RootSpan`] from a [`tracing::Span`].
    ///
    /// [`tracing::Span`]: https://docs.rs/tracing/0.1.40/tracing/struct.Span.html
    pub fn new(span: tracing::Span) -> Self {
        Self(span)
    }

    /// Get a reference to the underlying [`tracing::Span`].
    ///
    /// [`tracing::Span`]: https://docs.rs/tracing/0.1.40/tracing/struct.Span.html
    pub fn inner(&self) -> &tracing::Span {
        &self.0
    }

    /// Deconstruct the root span into its underlying [`tracing::Span`].
    ///
    /// [`tracing::Span`]: https://docs.rs/tracing/0.1.40/tracing/struct.Span.html
    pub fn into_inner(self) -> tracing::Span {
        self.0
    }
}
