//! Register constructors for the types that can be injected into your request and error handlers.  
//!
//! # Guide
//!
//! Check out the ["Dependency injection"](https://pavex.dev/docs/guide/dependency_injection)
//! section of Pavex's guide for a thorough introduction to dependency injection
//! in Pavex applications.
pub use lifecycle::Lifecycle;
use pavex_bp_schema::{Blueprint as BlueprintSchema, Component, Constructor};

use crate::blueprint::conversions::raw_callable2registered_callable;
use crate::blueprint::reflection::RawCallable;

mod lifecycle;

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
        let strategy = match strategy {
            CloningStrategy::NeverClone => pavex_bp_schema::CloningStrategy::NeverClone,
            CloningStrategy::CloneIfNecessary => pavex_bp_schema::CloningStrategy::CloneIfNecessary,
        };
        self.constructor().cloning_strategy = Some(strategy);
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
/// Determines whether Pavex is allowed to clone the output type returned by a constructor.
///
/// Check out [`Constructor::cloning`] for more information.
pub enum CloningStrategy {
    /// Pavex will **never** try clone the output type returned by the constructor.
    NeverClone,
    /// Pavex will only clone the output type returned by this constructor if it's
    /// necessary to generate code that satisfies Rust's borrow checker.
    CloneIfNecessary,
}
