use super::reflection::AnnotationCoordinates;
use crate::blueprint::ErrorHandler;
use crate::blueprint::conversions::coordinates2coordinates;
use pavex_bp_schema::{Blueprint as BlueprintSchema, Component, Location};

/// The input type for [`Blueprint::wrap`].
///
/// Check out [`Blueprint::wrap`] for more information on wrapping middlewares
/// in Pavex.
///
/// # Stability guarantees
///
/// Use the [`wrap`](macro@crate::wrap) attribute macro to create instances of `WrappingMiddleware`.\
/// `WrappingMiddleware`'s fields are an implementation detail of Pavex's macros and should not be relied upon:
/// newer versions of Pavex may add, remove or modify its fields.
///
/// [`Blueprint::wrap`]: crate::Blueprint::wrap
pub struct WrappingMiddleware {
    #[doc(hidden)]
    pub coordinates: AnnotationCoordinates,
}

/// The type returned by [`Blueprint::wrap`].
///
/// It allows you to further configure the behaviour of the registered wrapping
/// middleware.
///
/// [`Blueprint::wrap`]: crate::Blueprint::wrap
pub struct RegisteredWrappingMiddleware<'a> {
    pub(crate) blueprint: &'a mut BlueprintSchema,
    /// The index of the registered middleware in the blueprint's `components` vector.
    pub(crate) component_id: usize,
}

impl RegisteredWrappingMiddleware<'_> {
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
    /// use pavex::{error_handler, wrap, middleware::Next};
    /// use pavex::response::Response;
    /// use std::future::Future;
    /// # struct LogLevel;
    /// # struct Logger;
    /// # struct TimeoutError;
    ///
    /// // 👇 a fallible middleware
    /// #[wrap]
    /// pub fn timeout_middleware<C>(next: Next<C>) -> Result<Response, TimeoutError>
    /// where
    ///     C: Future<Output = Response>
    /// {
    ///     // [...]
    ///     # todo!()
    /// }
    ///
    /// #[error_handler]
    /// pub fn timeout_error_handler(
    ///     #[px(error_ref)] error: &TimeoutError,
    ///     log_level: LogLevel
    /// ) -> Response {
    ///     // [...]
    ///     # todo!()
    /// }
    ///
    /// # fn main() {
    /// let mut bp = Blueprint::new();
    /// bp.wrap(TIMEOUT_MIDDLEWARE)
    ///     .error_handler(TIMEOUT_ERROR_HANDLER);
    /// # }
    /// ```
    pub fn error_handler(mut self, error_handler: ErrorHandler) -> Self {
        let error_handler = pavex_bp_schema::ErrorHandler {
            coordinates: coordinates2coordinates(error_handler.coordinates),
            registered_at: Location::caller(),
        };
        self.wrapping_middleware().error_handler = Some(error_handler);
        self
    }

    fn wrapping_middleware(&mut self) -> &mut pavex_bp_schema::WrappingMiddleware {
        let component = &mut self.blueprint.components[self.component_id];
        let Component::WrappingMiddleware(c) = component else {
            unreachable!("The component should be a wrapping middleware")
        };
        c
    }
}
