//! Register constructors for the types that can be injected into your request and error handlers.  
//!
//! # Guide
//!
//! Check out the ["Dependency injection"](https://pavex.dev/docs/guide/dependency_injection)
//! section of Pavex's guide for a thorough introduction to dependency injection
//! in Pavex applications.
pub use lifecycle::Lifecycle;

use crate::blueprint::internals::RegisteredCallable;
use crate::blueprint::reflection::{RawCallable, RawCallableIdentifiers};
use crate::blueprint::Blueprint;

mod lifecycle;

/// The type returned by [`Blueprint::constructor`].
///
/// It allows you to further configure the behaviour of the registered constructor.
pub struct Constructor<'a> {
    pub(crate) blueprint: &'a mut Blueprint,
    /// The index of the registered constructor in the blueprint's `constructors` vector.
    pub(crate) constructor_id: usize,
}

impl<'a> Constructor<'a> {
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
    pub fn error_handler(self, error_handler: RawCallable) -> Self {
        let callable_identifiers = RawCallableIdentifiers::from_raw_callable(error_handler);
        let callable = RegisteredCallable {
            callable: callable_identifiers,
            location: std::panic::Location::caller().into(),
        };
        self.blueprint.constructors[self.constructor_id].error_handler = Some(callable);
        self
    }

    /// Set the cloning strategy for the output type returned by this constructor.
    ///
    /// By default,
    /// Pavex will **never** try to clone the output type returned by a constructor.  
    /// If the output type implements [`Clone`], you change the default by setting the cloning strategy
    /// to [`CloningStrategy::CloneIfNecessary`]: Pavex will clone the output type if
    /// it's necessary to generate code that satisfies Rust's borrow checker.
    pub fn cloning(self, strategy: CloningStrategy) -> Self {
        self.blueprint.constructors[self.constructor_id].cloning_strategy = Some(strategy);
        self
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
