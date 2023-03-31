//! Register constructors for the types that can be injected into your request and error handlers.  
//!
//! Check out [`Blueprint::constructor`] for a brief introduction to dependency injection in `pavex`.
//!
//! [`Blueprint::constructor`]: Blueprint::constructor
pub use lifecycle::Lifecycle;

use crate::reflection::{RawCallable, RawCallableIdentifiers};
use crate::Blueprint;

mod lifecycle;

/// The type returned by [`Blueprint::constructor`].
///
/// It allows you to further configure the behaviour of the registered constructor.
pub struct Constructor<'a> {
    pub(crate) blueprint: &'a mut Blueprint,
    pub(crate) constructor_identifiers: RawCallableIdentifiers,
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
    /// use pavex_builder::{Blueprint, f, constructor::Lifecycle};
    /// use pavex_runtime::{response::Response, hyper::body::Body};
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
    /// `pavex` will fail to generate the runtime code for your application if you register
    /// an error handler for an infallible constructor (i.e. a constructor that doesn't return
    /// a `Result`).
    pub fn error_handler(self, handler: RawCallable) -> Self {
        let callable_identifiers = RawCallableIdentifiers::new(handler.import_path);
        self.blueprint.error_handler_locations.insert(
            self.constructor_identifiers.clone(),
            std::panic::Location::caller().into(),
        );
        self.blueprint
            .constructors_error_handlers
            .insert(self.constructor_identifiers.clone(), callable_identifiers);
        self
    }
}
