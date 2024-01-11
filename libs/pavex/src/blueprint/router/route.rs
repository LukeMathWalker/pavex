use crate::blueprint::conversions::raw_callable2registered_callable;
use crate::blueprint::reflection::RawCallable;
use pavex_bp_schema::{Blueprint as BlueprintSchema, RegisteredComponent, RegisteredRoute};

/// The type returned by [`Blueprint::route`].
///
/// It allows you to further configure the behaviour of the registered route.
///
/// [`Blueprint::route`]: crate::blueprint::Blueprint::route
pub struct Route<'a> {
    pub(crate) blueprint: &'a mut BlueprintSchema,
    /// The index of the registered route in the blueprint's `components` vector.
    pub(crate) component_id: usize,
}

impl<'a> Route<'a> {
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
    /// use pavex::blueprint::{Blueprint, router::GET};
    /// use pavex::response::Response;
    /// # struct LogLevel;
    /// # struct RuntimeError;
    /// # struct ConfigurationError;
    ///
    /// fn request_handler() -> Result<Response, RuntimeError> {
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
    /// bp.route(GET, "/home", f!(crate::request_handler))
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
    pub fn error_handler(mut self, error_handler: RawCallable) -> Self {
        let callable = raw_callable2registered_callable(error_handler);
        self.route().error_handler = Some(callable);
        self
    }

    fn route(&mut self) -> &mut RegisteredRoute {
        let component = &mut self.blueprint.components[self.component_id];
        let RegisteredComponent::Route(c) = component else {
            unreachable!("The component should be a route")
        };
        c
    }
}
