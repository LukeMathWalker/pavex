use crate::blueprint::constructor::CloningStrategy;
use crate::blueprint::conversions::{cloning2cloning, raw_callable2registered_callable};
use crate::blueprint::reflection::RawCallable;
use pavex_bp_schema::Blueprint as BlueprintSchema;
use pavex_bp_schema::{Component, Constructor};

/// The type returned by [`Blueprint::constructor`].
///
/// It allows you to further configure the behaviour of the registered constructor.
///
/// [`Blueprint::constructor`]: crate::blueprint::Blueprint::constructor
pub struct RegisteredConstructor<'a> {
    pub(crate) blueprint: &'a mut BlueprintSchema,
    /// The index of the registered middleware in the blueprint's `constructors` vector.
    pub(crate) component_id: usize,
}

impl<'a> RegisteredConstructor<'a> {
    #[track_caller]
    /// Register an error handler.
    ///
    /// Error handlers convert the error type returned by your constructor into an HTTP response.
    ///
    /// Error handlers CANNOT consume the error type, they must take a reference to the
    /// error as input.  
    /// Error handlers can have additional input parameters alongside the error, as long as there
    /// are constructors registered for those parameter types.
    ///
    /// ```rust
    /// use pavex::f;
    /// use pavex::blueprint::{Blueprint, constructor::Lifecycle};
    /// use pavex::response::Response;
    /// # struct LogLevel;
    /// # struct Logger;
    /// # struct ConfigurationError;
    ///
    /// fn logger() -> Result<Logger, ConfigurationError> {
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
    /// bp.constructor(f!(crate::logger), Lifecycle::Transient)
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
    /// an error handler for an infallible constructor (i.e. a constructor that doesn't return
    /// a `Result`).
    pub fn error_handler(mut self, error_handler: RawCallable) -> Self {
        let callable = raw_callable2registered_callable(error_handler);
        self.constructor().error_handler = Some(callable);
        self
    }

    /// Set the cloning strategy for the output type returned by this constructor.
    ///
    /// By default,
    /// Pavex will **never** try to clone the output type returned by a constructor.  
    /// If the output type implements [`Clone`], you change the default by setting the cloning strategy
    /// to [`CloningStrategy::CloneIfNecessary`]: Pavex will clone the output type if
    /// it's necessary to generate code that satisfies Rust's borrow checker.
    pub fn cloning(mut self, strategy: CloningStrategy) -> Self {
        self.constructor().cloning_strategy = Some(cloning2cloning(strategy));
        self
    }

    fn constructor(&mut self) -> &mut Constructor {
        let component = &mut self.blueprint.components[self.component_id];
        let Component::Constructor(c) = component else {
            unreachable!("The component should be a constructor")
        };
        c
    }
}
