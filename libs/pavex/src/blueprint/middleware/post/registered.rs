use crate::blueprint::conversions::coordinates2coordinates;
use crate::blueprint::raw::RawErrorHandler;
use pavex_bp_schema::{
    Blueprint as BlueprintSchema, Component, ErrorHandler, Location, PostProcessingMiddleware,
};

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

impl RegisteredPostProcessingMiddleware<'_> {
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
    /// use pavex::blueprint::Blueprint;
    /// use pavex::{error_handler, post_process};
    /// use pavex::response::Response;
    /// # struct SizeError;
    ///
    /// // 👇 a fallible post-processing middleware
    /// #[post_process]
    /// pub fn max_response_size(response: Response) -> Result<Response, SizeError>
    /// {
    ///     // [...]
    ///     # todo!()
    /// }
    ///
    /// #[error_handler]
    /// pub fn size_error_handler(error: &SizeError) -> Response {
    ///     // [...]
    ///     # todo!()
    /// }
    ///
    /// # fn main() {
    /// let mut bp = Blueprint::new();
    /// bp.post_process(MAX_RESPONSE_SIZE)
    ///     .error_handler(SIZE_ERROR_HANDLER);
    /// # }
    /// ```
    pub fn error_handler(mut self, error_handler: RawErrorHandler) -> Self {
        let error_handler = ErrorHandler {
            coordinates: coordinates2coordinates(error_handler.coordinates),
            registered_at: Location::caller(),
        };
        self.post_processing_middleware().error_handler = Some(error_handler);
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
