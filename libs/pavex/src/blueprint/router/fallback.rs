use crate::blueprint::internals::RegisteredCallable;
use crate::blueprint::{
    reflection::{RawCallable, RawCallableIdentifiers},
    Blueprint,
};

/// The type returned by [`Blueprint::fallback`].
///
/// It allows you to further configure the behaviour of the registered handler.
pub struct Fallback<'a> {
    pub(crate) blueprint: &'a mut Blueprint,
}

impl<'a> Fallback<'a> {
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
    /// use pavex::f;
    /// use pavex::blueprint::Blueprint;
    /// use pavex::response::Response;
    /// # struct LogLevel;
    /// # struct RuntimeError;
    /// # struct ConfigurationError;
    ///
    /// fn fallback() -> Result<Response, RuntimeError> {
    ///     // [...]
    ///     # todo!()
    /// }
    ///
    /// fn error_to_response(error: &ConfigurationError, log_level: LogLevel) -> Response {
    ///     // [...]
    ///     # todo!()
    /// }
    ///
    /// # fn main() {
    /// let mut bp = Blueprint::new();
    /// bp.fallback(f!(crate::fallback))
    ///     .error_handler(f!(crate::error_to_response));
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
    pub fn error_handler(self, error_handler: RawCallable) -> Self {
        let callable_identifiers = RawCallableIdentifiers::from_raw_callable(error_handler);
        let callable = RegisteredCallable {
            callable: callable_identifiers,
            location: std::panic::Location::caller().into(),
        };
        if let Some(fallback) = &mut self.blueprint.fallback_request_handler {
            fallback.error_handler = Some(callable);
        }
        self
    }
}
