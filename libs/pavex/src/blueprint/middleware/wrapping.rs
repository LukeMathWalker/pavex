use crate::blueprint::{
    internals::RegisteredCallable,
    reflection::{RawCallable, RawCallableIdentifiers},
    Blueprint,
};

/// The type returned by [`Blueprint::wrap`].
///
/// It allows you to further configure the behaviour of the registered wrapping
/// middleware.
pub struct WrappingMiddleware<'a> {
    #[allow(dead_code)]
    pub(crate) blueprint: &'a mut Blueprint,
    /// The index of the registered wrapping middleware in the
    /// [`Blueprint`]'s `middlewares` vector.
    pub(crate) middleware_id: usize,
}

impl<'a> WrappingMiddleware<'a> {
    #[track_caller]
    /// Register an error handler.
    ///
    /// Error handlers convert the error type returned by your middleware into an HTTP response.
    ///
    /// Error handlers **can't** consume the error type, they must take a reference to the
    /// error as input.  
    /// Error handlers can have additional input parameters alongside the error, as long as there
    /// are constructors registered for those parameter types.
    ///
    /// ```rust
    /// use pavex::{f, blueprint::Blueprint, middleware::Next};
    /// use pavex::{response::Response, hyper::body::Body};
    /// use std::future::Future;
    /// # struct LogLevel;
    /// # struct Logger;
    /// # struct TimeoutError;
    ///
    /// fn timeout_middleware<C>(next: Next<C>) -> Result<Response, TimeoutError>
    /// where
    ///     C: Future<Output = Response>
    /// {
    ///     // [...]
    ///     # todo!()
    /// }
    ///
    /// fn error_to_response(error: &TimeoutError, log_level: LogLevel) -> Response {
    ///     // [...]
    ///     # todo!()
    /// }
    ///
    /// # fn main() {
    /// let mut bp = Blueprint::new();
    /// bp.wrap(f!(crate::timeout_middleware))
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
    /// an error handler for an infallible middleware (i.e. a middleware that doesn't return
    /// a `Result`).
    pub fn error_handler(self, error_handler: RawCallable) -> Self {
        let callable_identifiers = RawCallableIdentifiers::from_raw_callable(error_handler);
        let callable = RegisteredCallable {
            callable: callable_identifiers,
            location: std::panic::Location::caller().into(),
        };
        self.blueprint.middlewares[self.middleware_id].error_handler = Some(callable);
        self
    }
}
