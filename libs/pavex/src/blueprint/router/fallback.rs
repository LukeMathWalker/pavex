use crate::blueprint::conversions::raw_identifiers2callable;
use crate::blueprint::reflection::RawIdentifiers;
use crate::blueprint::{Blueprint, reflection::WithLocation};
use pavex_bp_schema::{Blueprint as BlueprintSchema, Callable, Component};

/// The type returned by [`Blueprint::fallback`].
///
/// It allows you to further configure the behaviour of the registered handler.
///
/// [`Blueprint::fallback`]: Blueprint::fallback
pub struct RegisteredFallback<'a> {
    pub(crate) blueprint: &'a mut BlueprintSchema,
    /// The index of the registered fallback in the blueprint's `components` vector.
    pub(crate) component_id: usize,
}

impl RegisteredFallback<'_> {
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
    /// use pavex::blueprint::Blueprint;
    /// use pavex::response::Response;
    /// # struct LogLevel;
    /// # struct RuntimeError;
    /// # struct ConfigurationError;
    ///
    /// fn fallback() -> Result<Response, RuntimeError> {
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
    /// bp.fallback(f!(crate::fallback))
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
    pub fn error_handler(mut self, error_handler: WithLocation<RawIdentifiers>) -> Self {
        let callable = raw_identifiers2callable(error_handler);
        self.fallback().error_handler = Some(callable);
        self
    }

    fn fallback(&mut self) -> &mut pavex_bp_schema::Fallback {
        let component = &mut self.blueprint.components[self.component_id];
        let Component::FallbackRequestHandler(fallback) = component else {
            unreachable!("The component should be a fallback request handler")
        };
        fallback
    }
}

/// A fallback that has been configured but has not yet been registered with a [`Blueprint`].
///
/// # Guide
///
/// Check out [`Blueprint::fallback`] for an introduction to fallback routes in Pavex.
///
/// # Use cases
///
/// [`Fallback`] is primarily used by
/// [kits](https://pavex.dev/docs/guide/dependency_injection/kits)
/// to allow users to customize (or disable!)
/// the bundled fallbacks **before** registering them with a [`Blueprint`].
#[derive(Clone, Debug)]
pub struct Fallback {
    pub(in crate::blueprint) callable: Callable,
    pub(in crate::blueprint) error_handler: Option<Callable>,
}

impl Fallback {
    /// Create a new (unregistered) fallback route.
    ///
    /// Check out the documentation of [`Blueprint::fallback`] for more details
    /// on fallback routes.
    #[track_caller]
    pub fn new(callable: WithLocation<RawIdentifiers>) -> Self {
        Self {
            callable: raw_identifiers2callable(callable),
            error_handler: None,
        }
    }

    /// Register an error handler for this fallback route.
    ///
    /// Check out the documentation of [`RegisteredFallback::error_handler`] for more details.
    #[track_caller]
    pub fn error_handler(mut self, error_handler: WithLocation<RawIdentifiers>) -> Self {
        self.error_handler = Some(raw_identifiers2callable(error_handler));
        self
    }

    /// Register this fallback route with a [`Blueprint`].
    ///
    /// Check out the documentation of [`Blueprint::fallback`] for more details.
    pub fn register(self, bp: &mut Blueprint) -> RegisteredFallback {
        bp.register_fallback(self)
    }
}
