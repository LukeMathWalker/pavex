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
/// # Accessing `RootSpan`
///
/// You can leverage [Pavex's dependency injection system](https://pavex.dev/docs/guide/dependency_injection/)
/// to access the root span from any of your components—request handlers,
/// middlewares, error observers, etc.  
/// It's enough to add an input parameter with type `&RootSpan` to your component.
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
/// For this very reason, `pavex new` adds a constructor for [`RootSpan`] **in your own project**,
/// which you are then free to customize and tailor to your own needs when the time comes.
///
/// # Why is `RootSpan` defined in `pavex_tracing`?
///
/// You may wonder: what's the purpose of adding `RootSpan` to `pavex_tracing` then?
/// Why don't we define `RootSpan` inside the template project, like its "default" constructor?
///
/// You could!  
/// By moving `RootSpan` inside `pavex_tracing`, though, we can **standardize on the same type**.  
/// For example, a third-party crate could provide a middleware to populate the `status_code` field on
/// `RootSpan`: you can pull it into your project and it'll work, since it operates on the very
/// same `RootSpan` type.
/// That wouldn't be the case if `RootSpan` was defined in your own project: **every single piece
/// of telemetry logic would have to be bespoke**.
/// Not ideal.
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
