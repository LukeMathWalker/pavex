use crate::blueprint::ErrorHandler;
use crate::blueprint::Lint;
use crate::blueprint::conversions::{
    cloning2cloning, coordinates2coordinates, lifecycle2lifecycle, lint2lint,
};
use pavex_bp_schema::Component;
use pavex_bp_schema::{Blueprint as BlueprintSchema, LintSetting, Location};

use super::CloningPolicy;
use super::Lifecycle;
use super::reflection::AnnotationCoordinates;

/// The input type for [`Blueprint::constructor`].
///
/// Check out [`Blueprint::constructor`] for more information on dependency injection
/// in Pavex.
///
/// # Stability guarantees
///
/// Use one of Pavex's constructor attributes (
/// [`singleton`](macro@crate::singleton), [`request_scoped`](macro@crate::request_scoped), or [`transient`](macro@crate::transient))
/// to create instances of `Constructor`.\
/// `Constructor`'s fields are an implementation detail of Pavex's macros and should not be relied upon:
/// newer versions of Pavex may add, remove or modify its fields.
///
/// [`Blueprint::constructor`]: crate::Blueprint::constructor
pub struct Constructor {
    #[doc(hidden)]
    pub coordinates: AnnotationCoordinates,
}

/// A constructor registered via [`Blueprint::constructor`].
///
/// # Example
///
/// You can use the methods exposed by [`RegisteredConstructor`] to tune the behaviour
/// of the registered constructor type.
/// For example, instruct Pavex to clone the constructed type if it's necessary to satisfy
/// the borrow checker:
///
/// ```rust
/// use pavex::{methods, Blueprint};
///
/// # pub struct PoolConfig;
/// pub struct Pool {
///     // [...]
/// }
///
/// #[methods]
/// impl Pool {
///     #[singleton]
///     pub fn new(config: &PoolConfig) -> Self {
///         # todo!()
///         // [...]
///     }
/// }
///
/// let mut bp = Blueprint::new();
/// // This is equivalent to `#[singleton(clone_if_necessary)]`
/// bp.constructor(POOL_NEW).clone_if_necessary();
/// ```
///
/// # Example: override the annotation
///
/// You can also override the behaviour specified via the [`singleton`](macro@crate::singleton) attribute.
///
/// ```rust
/// use pavex::{methods, Blueprint};
///
/// # pub struct PoolConfig;
/// pub struct Pool {
///     // [...]
/// }
///
/// #[methods]
/// impl Pool {
///     #[singleton(clone_if_necessary)]
///     pub fn new(config: &PoolConfig) -> Self {
///         # todo!()
///         // [...]
///     }
/// }
///
/// let mut bp = Blueprint::new();
/// // Using `never_clone` here, we are overriding the `clone_if_necessary`
/// // flag specified via the `singleton` attribute.
/// // This is equivalent to `#[singleton]`, thus restoring
/// // the default behaviour.
/// bp.constructor(POOL_NEW).never_clone();
/// ```
///
/// [`Blueprint::constructor`]: crate::Blueprint::constructor
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
    /// use pavex::Blueprint;
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
    pub fn error_handler(mut self, error_handler: ErrorHandler) -> Self {
        let error_handler = pavex_bp_schema::ErrorHandler {
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
    pub fn cloning(mut self, strategy: CloningPolicy) -> Self {
        self.constructor().cloning_policy = Some(cloning2cloning(strategy));
        self
    }

    /// Set the cloning strategy to [`CloningPolicy::CloneIfNecessary`].
    /// Check out [`cloning`](Self::cloning) method for more details.
    pub fn clone_if_necessary(self) -> Self {
        self.cloning(CloningPolicy::CloneIfNecessary)
    }

    /// Set the cloning strategy to [`CloningPolicy::NeverClone`].
    /// Check out [`cloning`](Self::cloning) method for more details.
    pub fn never_clone(self) -> Self {
        self.cloning(CloningPolicy::NeverClone)
    }

    /// Silence a specific [`Lint`] for this constructor.
    pub fn allow(mut self, lint: Lint) -> Self {
        self.constructor()
            .lints
            .insert(lint2lint(lint), LintSetting::Ignore);
        self
    }

    /// Fail the build if a specific [`Lint`] triggers
    /// for this constructor.
    pub fn deny(mut self, lint: Lint) -> Self {
        self.constructor()
            .lints
            .insert(lint2lint(lint), LintSetting::Enforce);
        self
    }

    fn constructor(&mut self) -> &mut pavex_bp_schema::Constructor {
        let component = &mut self.blueprint.components[self.component_id];
        let Component::Constructor(c) = component else {
            unreachable!("The component should be a constructor")
        };
        c
    }
}
