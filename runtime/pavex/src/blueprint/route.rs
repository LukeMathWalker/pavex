use crate::blueprint::ErrorHandler;
use crate::blueprint::conversions::coordinates2coordinates;
use pavex_bp_schema::{Blueprint as BlueprintSchema, Component, Location};

use super::reflection::AnnotationCoordinates;

/// The input type for [`Blueprint::route`].
///
/// Check out [`Blueprint::route`] for more information on request routing
/// in Pavex.
///
/// # Stability guarantees
///
/// Use one of Pavex's route attributes (
/// [`route`](macro@crate::route) or a method-specific one, like [`get`](macro@crate::get) or [`post`](macro@crate::post))
/// to create instances of `Route`.\
/// `Route`'s fields are an implementation detail of Pavex's macros and should not be relied upon:
/// newer versions of Pavex may add, remove or modify its fields.
///
/// [`Blueprint::route`]: crate::Blueprint::route
pub struct Route {
    #[doc(hidden)]
    pub coordinates: AnnotationCoordinates,
}

/// The type returned by [`Blueprint::route`].
///
/// It allows you to further configure the behaviour of the registered route.
///
/// [`Blueprint::route`]: crate::Blueprint::route
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
    /// use pavex::Blueprint;
    /// use pavex::Response;
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
    pub fn error_handler(mut self, error_handler: ErrorHandler) -> Self {
        let error_handler = pavex_bp_schema::ErrorHandler {
            coordinates: coordinates2coordinates(error_handler.coordinates),
            registered_at: Location::caller(),
        };
        self.route().error_handler = Some(error_handler);
        self
    }

    fn route(&mut self) -> &mut pavex_bp_schema::Route {
        let component = &mut self.blueprint.components[self.component_id];
        let Component::Route(c) = component else {
            unreachable!("The component should be a route")
        };
        c
    }
}
