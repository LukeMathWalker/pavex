use crate::blueprint::conversions::raw_callable2registered_callable;
use crate::blueprint::reflection::RawCallable;
use pavex_bp_schema::{
    Blueprint as BlueprintSchema, Component, PostProcessingMiddleware, WrappingMiddleware,
};

/// The type returned by [`Blueprint::wrap`].
///
/// It allows you to further configure the behaviour of the registered wrapping
/// middleware.
///
/// [`Blueprint::wrap`]: crate::blueprint::Blueprint::wrap
pub struct RegisteredWrappingMiddleware<'a> {
    pub(crate) blueprint: &'a mut BlueprintSchema,
    /// The index of the registered middleware in the blueprint's `components` vector.
    pub(crate) component_id: usize,
}

impl<'a> RegisteredWrappingMiddleware<'a> {
    #[track_caller]
    /// Register an error handler.
    ///
    /// If an error handler has already been registered for this middleware, it will be
    /// overwritten.
    ///
    /// # Guide
    ///
    /// Check out the ["Error handlers"](https://pavex.dev/docs/guide/errors/error_handlers)
    /// section of Pavex's guide for a thorough introduction to error handlers
    /// in Pavex applications.
    ///
    /// # Example
    ///
    /// ```rust
    /// use pavex::{f, blueprint::Blueprint, middleware::Next};
    /// use pavex::response::Response;
    /// use std::future::Future;
    /// # struct LogLevel;
    /// # struct Logger;
    /// # struct TimeoutError;
    ///
    /// // 👇 a fallible middleware
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
    pub fn error_handler(mut self, error_handler: RawCallable) -> Self {
        let callable = raw_callable2registered_callable(error_handler);
        self.wrapping_middleware().error_handler = Some(callable);
        self
    }

    fn wrapping_middleware(&mut self) -> &mut WrappingMiddleware {
        let component = &mut self.blueprint.components[self.component_id];
        let Component::WrappingMiddleware(c) = component else {
            unreachable!("The component should be a wrapping middleware")
        };
        c
    }
}

/// The type returned by [`Blueprint::post_process`].
///
/// It allows you to further configure the behaviour of post-processing middleware
/// you just registered.
///
/// [`Blueprint::post_process`]: crate::blueprint::Blueprint::post_process
pub struct RegisteredPostProcessingMiddleware<'a> {
    pub(crate) blueprint: &'a mut BlueprintSchema,
    /// The index of the registered middleware in the blueprint's `components` vector.
    pub(crate) component_id: usize,
}

impl<'a> RegisteredPostProcessingMiddleware<'a> {
    #[track_caller]
    /// Register an error handler.
    ///
    /// If an error handler has already been registered for this middleware, it will be
    /// overwritten.
    ///
    /// # Guide
    ///
    /// Check out the ["Error handlers"](https://pavex.dev/docs/guide/errors/error_handlers)
    /// section of Pavex's guide for a thorough introduction to error handlers
    /// in Pavex applications.
    ///
    /// # Example
    ///
    /// ```rust
    /// use pavex::{f, blueprint::Blueprint};
    /// use pavex::response::Response;
    /// # struct SizeError;
    ///
    /// // 👇 a fallible post-processing middleware
    /// fn max_response_size(response: Response) -> Result<Response, SizeError>
    /// {
    ///     // [...]
    ///     # todo!()
    /// }
    ///
    /// fn error_to_response(error: &SizeError) -> Response {
    ///     // [...]
    ///     # todo!()
    /// }
    ///
    /// # fn main() {
    /// let mut bp = Blueprint::new();
    /// bp.post_process(f!(crate::max_response_size))
    ///     .error_handler(f!(crate::error_to_response));
    /// # }
    /// ```
    pub fn error_handler(mut self, error_handler: RawCallable) -> Self {
        let callable = raw_callable2registered_callable(error_handler);
        self.post_processing_middleware().error_handler = Some(callable);
        self
    }

    fn post_processing_middleware(&mut self) -> &mut PostProcessingMiddleware {
        let component = &mut self.blueprint.components[self.component_id];
        let Component::PostProcessingMiddleware(c) = component else {
            unreachable!("The component should be a post-processing middleware")
        };
        c
    }
}
