use super::reflection::AnnotationCoordinates;
use crate::blueprint::ErrorHandler;
use crate::blueprint::conversions::coordinates2coordinates;
use pavex_bp_schema::{Blueprint as BlueprintSchema, Component, Location};

/// The input type for [`Blueprint::pre_process`].
///
/// Check out [`Blueprint::pre_process`] for more information on pre-processing middlewares
/// in Pavex.
///
/// # Stability guarantees
///
/// Use the [`pre_process`](macro@crate::pre_process) attribute macro to create instances of `PreProcessingMiddleware`.\
/// `PreProcessingMiddleware`'s fields are an implementation detail of Pavex's macros and should not be relied upon:
/// newer versions of Pavex may add, remove or modify its fields.
///
/// [`Blueprint::pre_process`]: crate::Blueprint::pre_process
pub struct PreProcessingMiddleware {
    #[doc(hidden)]
    pub coordinates: AnnotationCoordinates,
}

/// The type returned by [`Blueprint::pre_process`].
///
/// It allows you to further configure the behaviour of the registered pre-processing
/// middleware.
///
/// [`Blueprint::pre_process`]: crate::Blueprint::pre_process
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
    /// use pavex::Blueprint;
    /// use pavex::{error_handler, pre_process, middleware::Processing};
    /// use pavex::request::RequestHead;
    /// use pavex::response::Response;
    /// # struct LogLevel;
    /// # struct AuthError;
    ///
    /// // 👇 a fallible middleware
    /// #[pre_process]
    /// pub fn reject_anonymous(request_head: &RequestHead) -> Result<Processing, AuthError>
    /// {
    ///     // [...]
    ///     # todo!()
    /// }
    ///
    /// #[error_handler]
    /// pub fn auth_error_handler(
    ///     #[px(error_ref)] error: &AuthError,
    ///     log_level: LogLevel
    /// ) -> Response {
    ///     // [...]
    ///     # todo!()
    /// }
    ///
    /// # fn main() {
    /// let mut bp = Blueprint::new();
    /// bp.pre_process(REJECT_ANONYMOUS)
    ///     .error_handler(AUTH_ERROR_HANDLER);
    /// # }
    /// ```
    pub fn error_handler(mut self, error_handler: ErrorHandler) -> Self {
        let error_handler = pavex_bp_schema::ErrorHandler {
            coordinates: coordinates2coordinates(error_handler.coordinates),
            registered_at: Location::caller(),
        };
        self.pre_processing_middleware().error_handler = Some(error_handler);
        self
    }

    fn pre_processing_middleware(&mut self) -> &mut pavex_bp_schema::PreProcessingMiddleware {
        let component = &mut self.blueprint.components[self.component_id];
        let Component::PreProcessingMiddleware(c) = component else {
            unreachable!("The component should be a pre-processing middleware")
        };
        c
    }
}
