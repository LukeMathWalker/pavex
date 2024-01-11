use crate::blueprint::conversions::raw_callable2registered_callable;
use crate::blueprint::reflection::RawCallable;
use pavex_bp_schema::{
    Blueprint as BlueprintSchema, RegisteredComponent, RegisteredWrappingMiddleware,
};

/// The type returned by [`Blueprint::wrap`].
///
/// It allows you to further configure the behaviour of the registered wrapping
/// middleware.
///
/// [`Blueprint::wrap`]: crate::blueprint::Blueprint::wrap
pub struct WrappingMiddleware<'a> {
    pub(crate) blueprint: &'a mut BlueprintSchema,
    /// The index of the registered middleware in the blueprint's `components` vector.
    pub(crate) component_id: usize,
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
    /// use pavex::response::Response;
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
    pub fn error_handler(mut self, error_handler: RawCallable) -> Self {
        let callable = raw_callable2registered_callable(error_handler);
        self.wrapping_middleware().error_handler = Some(callable);
        self
    }

    fn wrapping_middleware(&mut self) -> &mut RegisteredWrappingMiddleware {
        let component = &mut self.blueprint.components[self.component_id];
        let RegisteredComponent::WrappingMiddleware(c) = component else {
            unreachable!("The component should be a wrapping middleware")
        };
        c
    }
}
