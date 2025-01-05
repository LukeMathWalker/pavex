use crate::blueprint::conversions::raw_identifiers2callable;
use crate::blueprint::reflection::RawIdentifiers;
use pavex_bp_schema::{Blueprint as BlueprintSchema, Component, PreProcessingMiddleware};

/// The type returned by [`Blueprint::pre_process`].
///
/// It allows you to further configure the behaviour of the registered pre-processing
/// middleware.
///
/// [`Blueprint::pre_process`]: crate::blueprint::Blueprint::pre_process
pub struct RegisteredPreProcessingMiddleware<'a> {
    pub(crate) blueprint: &'a mut BlueprintSchema,
    /// The index of the registered middleware in the blueprint's `components` vector.
    pub(crate) component_id: usize,
}

impl RegisteredPreProcessingMiddleware<'_> {
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
    /// use pavex::{f, blueprint::Blueprint, middleware::Processing};
    /// use pavex::request::RequestHead;
    /// use pavex::response::Response;
    /// # struct LogLevel;
    /// # struct AuthError;
    ///
    /// // 👇 a fallible middleware
    /// fn reject_anonymous(request_head: &RequestHead) -> Result<Processing, AuthError>
    /// {
    ///     // [...]
    ///     # todo!()
    /// }
    ///
    /// fn error_to_response(error: &AuthError, log_level: LogLevel) -> Response {
    ///     // [...]
    ///     # todo!()
    /// }
    ///
    /// # fn main() {
    /// let mut bp = Blueprint::new();
    /// bp.wrap(f!(crate::reject_anonymous))
    ///     .error_handler(f!(crate::error_to_response));
    /// # }
    /// ```
    pub fn error_handler(mut self, error_handler: RawIdentifiers) -> Self {
        let callable = raw_identifiers2callable(error_handler);
        self.pre_processing_middleware().error_handler = Some(callable);
        self
    }

    fn pre_processing_middleware(&mut self) -> &mut PreProcessingMiddleware {
        let component = &mut self.blueprint.components[self.component_id];
        let Component::PreProcessingMiddleware(c) = component else {
            unreachable!("The component should be a pre-processing middleware")
        };
        c
    }
}
