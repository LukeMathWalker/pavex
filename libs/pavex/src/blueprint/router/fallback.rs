use crate::blueprint::conversions::coordinates2coordinates;
use crate::blueprint::raw::RawErrorHandler;
use pavex_bp_schema::{Blueprint as BlueprintSchema, Component, ErrorHandler, Fallback, Location};

/// The type returned by [`Blueprint::fallback`].
///
/// It allows you to further configure the behaviour of the registered handler.
///
/// [`Blueprint::fallback`]: Blueprint::fallback
pub struct RegisteredFallback<'a> {
    pub(crate) blueprint: &'a mut BlueprintSchema,
    /// The index of the registered fallback in the blueprint's `components` vector.
    pub(crate) component_id: usize,
}

impl RegisteredFallback<'_> {
    #[track_caller]
    /// Register an error handler.
    ///
    /// Error handlers convert the error type returned by your request handler into an HTTP response.
    ///
    /// Error handlers CANNOT consume the error type, they must take a reference to the
    /// error as input.
    /// Error handlers can have additional input parameters alongside the error, as long as there
    /// are constructors registered for those parameter types.
    ///
    /// ```rust
    /// use pavex::blueprint::Blueprint;
    /// use pavex::{error_handler, fallback};
    /// use pavex::response::Response;
    /// # struct LogLevel;
    /// # struct RuntimeError;
    ///
    /// // 👇 a fallible fallback handler
    /// #[fallback]
    /// pub fn fallback_handler() -> Result<Response, RuntimeError> {
    ///     // [...]
    ///     # todo!()
    /// }
    ///
    /// #[error_handler]
    /// pub fn runtime_error_handler(
    ///     #[px(error_ref)] error: &RuntimeError,
    ///     log_level: LogLevel
    /// ) -> Response {
    ///     // [...]
    ///     # todo!()
    /// }
    ///
    /// # fn main() {
    /// let mut bp = Blueprint::new();
    /// bp.fallback(FALLBACK_HANDLER)
    ///     .error_handler(RUNTIME_ERROR_HANDLER);
    /// # }
    /// ```
    ///
    /// If an error handler has already been registered for the same error type, it will be
    /// overwritten.
    ///
    /// ## Common Errors
    ///
    /// Pavex will fail to generate the runtime code for your application if you register
    /// an error handler for an infallible request handler (i.e. a request handler that doesn't
    /// return a `Result`).
    pub fn error_handler(mut self, error_handler: RawErrorHandler) -> Self {
        let error_handler = ErrorHandler {
            coordinates: coordinates2coordinates(error_handler.coordinates),
            registered_at: Location::caller(),
        };
        self.fallback().error_handler = Some(error_handler);
        self
    }

    fn fallback(&mut self) -> &mut Fallback {
        let component = &mut self.blueprint.components[self.component_id];
        let Component::FallbackRequestHandler(fallback) = component else {
            unreachable!("The component should be a fallback request handler")
        };
        fallback
    }
}
