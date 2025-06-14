use crate::blueprint::conversions::coordinates2coordinates;
use crate::blueprint::raw::RawErrorHandler;
use pavex_bp_schema::{Blueprint as BlueprintSchema, Component, ErrorHandler, Location, Route};

/// The type returned by [`Blueprint::route`].
///
/// It allows you to further configure the behaviour of the registered route.
///
/// [`Blueprint::route`]: crate::blueprint::Blueprint::route
pub struct RegisteredRoute<'a> {
    pub(crate) blueprint: &'a mut BlueprintSchema,
    /// The index of the registered route in the blueprint's `components` vector.
    pub(crate) component_id: usize,
}

impl RegisteredRoute<'_> {
    #[track_caller]
    /// Register an error handler.
    ///
    /// If an error handler has already been registered for this route, it will be
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
    /// use pavex::{get, error_handler};
    /// use pavex::blueprint::Blueprint;
    /// use pavex::response::Response;
    /// # struct ConfigError;
    ///
    /// // 👇 a fallible request handler
    /// #[get(path = "/home")]
    /// pub fn get_home() -> Result<Response, ConfigError> {
    ///     // [...]
    ///     # todo!()
    /// }
    ///
    /// #[error_handler(default = false)]
    /// pub fn config_error_handler(error: &ConfigError) -> Response {
    ///     // [...]
    ///     # todo!()
    /// }
    ///
    /// # fn main() {
    /// let mut bp = Blueprint::new();
    /// bp.route(GET_HOME).error_handler(CONFIG_ERROR_HANDLER);
    /// # }
    /// ```
    pub fn error_handler(mut self, error_handler: RawErrorHandler) -> Self {
        let error_handler = ErrorHandler {
            coordinates: coordinates2coordinates(error_handler.coordinates),
            registered_at: Location::caller(),
        };
        self.route().error_handler = Some(error_handler);
        self
    }

    fn route(&mut self) -> &mut Route {
        let component = &mut self.blueprint.components[self.component_id];
        let Component::Route(c) = component else {
            unreachable!("The component should be a route")
        };
        c
    }
}
