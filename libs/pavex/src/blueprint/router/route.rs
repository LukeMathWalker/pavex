use crate::blueprint::conversions::raw_identifiers2callable;
use crate::blueprint::reflection::RawIdentifiers;
use crate::blueprint::router::MethodGuard;
use crate::blueprint::Blueprint;
use pavex_bp_schema::{Blueprint as BlueprintSchema, Callable, Component};

/// The type returned by [`Blueprint::route`].
///
/// It allows you to further configure the behaviour of the registered route.
///
/// [`Blueprint::route`]: Blueprint::route
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
    /// use pavex::f;
    /// use pavex::blueprint::{Blueprint, router::GET};
    /// use pavex::response::Response;
    /// # struct LogLevel;
    /// # struct RuntimeError;
    /// # struct ConfigurationError;
    ///
    /// // 👇 a fallible request handler
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
    pub fn error_handler(mut self, error_handler: RawIdentifiers) -> Self {
        let callable = raw_identifiers2callable(error_handler);
        self.route().error_handler = Some(callable);
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

/// A route that has been configured but has not yet been registered with a [`Blueprint`].
///
/// # Guide
///
/// Check out the ["Routing"](https://pavex.dev/docs/guide/routing) section of Pavex's guide
/// for a thorough introduction to routing in Pavex applications.
///
/// # Use cases
///
/// [`Route`] is primarily used by
/// [kits](https://pavex.dev/docs/guide/dependency_injection/kits)
/// to allow users to customize (or disable!)
/// the bundled routes **before** registering them with a [`Blueprint`].
#[derive(Clone, Debug)]
pub struct Route {
    pub(in crate::blueprint) method_guard: MethodGuard,
    pub(in crate::blueprint) path: String,
    pub(in crate::blueprint) callable: Callable,
    pub(in crate::blueprint) error_handler: Option<Callable>,
}

impl Route {
    /// Create a new (unregistered) route.
    ///
    /// Check out the documentation of [`Blueprint::route`] for more details
    /// on routes.
    #[track_caller]
    pub fn new(method_guard: MethodGuard, path: &str, callable: RawIdentifiers) -> Self {
        Self {
            callable: raw_identifiers2callable(callable),
            error_handler: None,
            method_guard,
            path: path.to_owned(),
        }
    }

    /// Register an error handler for this route.
    ///
    /// Check out the documentation of [`RegisteredRoute::error_handler`] for more details.
    #[track_caller]
    pub fn error_handler(mut self, error_handler: RawIdentifiers) -> Self {
        self.error_handler = Some(raw_identifiers2callable(error_handler));
        self
    }

    /// Register this route with a [`Blueprint`].
    ///
    /// Check out the documentation of [`Blueprint::route`] for more details.
    pub fn register(self, bp: &mut Blueprint) -> RegisteredRoute {
        bp.register_route(self)
    }
}
