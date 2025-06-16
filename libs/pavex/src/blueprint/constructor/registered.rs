use crate::blueprint::constructor::CloningStrategy;
use crate::blueprint::conversions::{
    cloning2cloning, coordinates2coordinates, lifecycle2lifecycle, lint2lint,
};
use crate::blueprint::linter::Lint;
use crate::blueprint::raw::RawErrorHandler;
use pavex_bp_schema::{Blueprint as BlueprintSchema, ErrorHandler, LintSetting, Location};
use pavex_bp_schema::{Component, Constructor};

use super::Lifecycle;

/// The type returned by [`Blueprint::constructor`].
///
/// It allows you to further configure the behaviour of the registered constructor.
///
/// [`Blueprint::constructor`]: crate::blueprint::Blueprint::constructor
pub struct RegisteredConstructor<'a> {
    pub(crate) blueprint: &'a mut BlueprintSchema,
    /// The index of the registered constructor in the blueprint's `components` vector.
    pub(crate) component_id: usize,
}

impl RegisteredConstructor<'_> {
    #[track_caller]
    /// Register an error handler.
    ///
    /// If an error handler has already been registered for this constructor, it will be
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
    /// use pavex::blueprint::Blueprint;
    /// use pavex::response::Response;
    /// use pavex::{methods, transient};
    /// # struct LogLevel;
    /// # struct Logger;
    /// # struct ConfigurationError;
    ///
    /// // 👇 a fallible constructor
    /// #[transient]
    /// pub fn logger() -> Result<Logger, ConfigurationError> {
    ///     // [...]
    ///     # todo!()
    /// }
    ///
    /// #[methods]
    /// impl ConfigurationError {
    ///     #[error_handler]
    ///     fn to_response(
    ///         #[px(error_ref)] &self,
    ///         log_level: LogLevel,
    ///     ) -> Response {
    ///         // [...]
    ///         # todo!()
    ///     }
    /// }
    ///
    /// # fn main() {
    /// let mut bp = Blueprint::new();
    /// bp.constructor(LOGGER)
    ///     .error_handler(CONFIGURATION_ERROR_TO_RESPONSE);
    /// # }
    /// ```
    pub fn error_handler(mut self, error_handler: RawErrorHandler) -> Self {
        let error_handler = ErrorHandler {
            coordinates: coordinates2coordinates(error_handler.coordinates),
            registered_at: Location::caller(),
        };
        self.constructor().error_handler = Some(error_handler);
        self
    }

    /// Change the constructor lifecycle.
    pub fn lifecycle(mut self, lifecycle: Lifecycle) -> Self {
        self.constructor().lifecycle = Some(lifecycle2lifecycle(lifecycle));
        self
    }

    /// Set the cloning strategy for the output type returned by this constructor.
    ///
    /// By default,
    /// Pavex will **never** try to clone the output type returned by a constructor.
    /// If the output type implements [`Clone`], you can change the default by invoking
    /// [`clone_if_necessary`](Self::clone_if_necessary): Pavex will clone the output type if
    /// it's necessary to generate code that satisfies Rust's borrow checker.
    pub fn cloning(mut self, strategy: CloningStrategy) -> Self {
        self.constructor().cloning_strategy = Some(cloning2cloning(strategy));
        self
    }

    /// Set the cloning strategy to [`CloningStrategy::CloneIfNecessary`].
    /// Check out [`cloning`](Self::cloning) method for more details.
    pub fn clone_if_necessary(self) -> Self {
        self.cloning(CloningStrategy::CloneIfNecessary)
    }

    /// Set the cloning strategy to [`CloningStrategy::NeverClone`].
    /// Check out [`cloning`](Self::cloning) method for more details.
    pub fn never_clone(self) -> Self {
        self.cloning(CloningStrategy::NeverClone)
    }

    /// Tell Pavex to ignore a specific [`Lint`] when analysing
    /// this constructor and the way it's used.
    pub fn ignore(mut self, lint: Lint) -> Self {
        self.constructor()
            .lints
            .insert(lint2lint(lint), LintSetting::Ignore);
        self
    }

    /// Tell Pavex to enforce a specific [`Lint`] when analysing
    /// this constructor and the way it's used.
    pub fn enforce(mut self, lint: Lint) -> Self {
        self.constructor()
            .lints
            .insert(lint2lint(lint), LintSetting::Enforce);
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
