use crate::blueprint::constructor::{CloningStrategy, Lifecycle, RegisteredConstructor};
use crate::blueprint::conversions::{lint2lint, raw_identifiers2callable};
use crate::blueprint::linter::Lint;
use crate::blueprint::reflection::RawIdentifiers;
use crate::blueprint::Blueprint;
use pavex_bp_schema::{Callable, LintSetting};
use std::collections::BTreeMap;

/// A constructor that has been configured but has not yet been registered with a [`Blueprint`].
///
/// # Guide
///
/// Check out the ["Dependency injection"](https://pavex.dev/docs/guide/dependency_injection)
/// section of Pavex's guide for a thorough introduction to dependency injection
/// in Pavex applications.
///
/// # Use cases
///
/// [`Constructor`] is primarily used by kits (e.g. [`ApiKit`](crate::kit::ApiKit))
/// to allow users to customize (or disable!)
/// the bundled constructors **before** registering them with a [`Blueprint`].
#[derive(Clone, Debug)]
pub struct Constructor {
    pub(in crate::blueprint) callable: Callable,
    pub(in crate::blueprint) lifecycle: Lifecycle,
    pub(in crate::blueprint) cloning_strategy: Option<CloningStrategy>,
    pub(in crate::blueprint) error_handler: Option<Callable>,
    pub(in crate::blueprint) lints: BTreeMap<pavex_bp_schema::Lint, LintSetting>,
}

impl Constructor {
    /// Create a new (unregistered) constructor.
    ///
    /// Check out the documentation of [`Blueprint::constructor`] for more details
    /// on constructors.
    #[track_caller]
    pub fn new(callable: RawIdentifiers, lifecycle: Lifecycle) -> Self {
        Self {
            callable: raw_identifiers2callable(callable),
            lifecycle,
            cloning_strategy: None,
            error_handler: None,
            lints: Default::default(),
        }
    }

    /// Create a new (unregistered) constructor with a [singleton](Lifecycle::Singleton) lifecycle.
    ///
    /// It's a shorthand for [`Constructor::new(callable, Lifecycle::Singleton)`](Constructor::new).
    #[track_caller]
    pub fn singleton(callable: RawIdentifiers) -> Self {
        Constructor::new(callable, Lifecycle::Singleton)
    }

    /// Create a new (unregistered) constructor with a [request-scoped](Lifecycle::RequestScoped) lifecycle.
    ///
    /// It's a shorthand for [`Constructor::new(callable, Lifecycle::RequestScoped)`](Constructor::new).
    #[track_caller]
    pub fn request_scoped(callable: RawIdentifiers) -> Self {
        Constructor::new(callable, Lifecycle::RequestScoped)
    }

    /// Create a new (unregistered) constructor with a [transient](Lifecycle::Transient) lifecycle.
    ///
    /// It's a shorthand for [`Constructor::new(callable, Lifecycle::Transient)`](Constructor::new).
    #[track_caller]
    pub fn transient(callable: RawIdentifiers) -> Self {
        Constructor::new(callable, Lifecycle::Transient)
    }

    /// Register an error handler for this constructor.
    ///
    /// Check out the documentation of [`RegisteredConstructor::error_handler`] for more details.
    #[track_caller]
    pub fn error_handler(mut self, error_handler: RawIdentifiers) -> Self {
        self.error_handler = Some(raw_identifiers2callable(error_handler));
        self
    }

    /// Set the cloning strategy for the output type returned by this constructor.
    ///
    /// Check out the documentation of [`CloningStrategy`] for more details.
    pub fn cloning(mut self, cloning_strategy: CloningStrategy) -> Self {
        self.cloning_strategy = Some(cloning_strategy);
        self
    }

    /// Set the cloning strategy to [`CloningStrategy::CloneIfNecessary`].  
    /// Check out [`Constructor::cloning`] for more details.
    pub fn clone_if_necessary(self) -> Self {
        self.cloning(CloningStrategy::CloneIfNecessary)
    }

    /// Set the cloning strategy to [`CloningStrategy::NeverClone`].  
    /// Check out [`Constructor::cloning`] for more details.
    pub fn never_clone(self) -> Self {
        self.cloning(CloningStrategy::NeverClone)
    }

    /// Tell Pavex to ignore a specific [`Lint`] when analysing
    /// this constructor and the way it's used.
    pub fn ignore(mut self, lint: Lint) -> Self {
        self.lints.insert(lint2lint(lint), LintSetting::Ignore);
        self
    }

    /// Tell Pavex to enforce a specific [`Lint`] when analysing
    /// this constructor and the way it's used.
    pub fn enforce(mut self, lint: Lint) -> Self {
        self.lints.insert(lint2lint(lint), LintSetting::Enforce);
        self
    }

    /// Register this constructor with a [`Blueprint`].
    ///
    /// Check out the documentation of [`Blueprint::constructor`] for more details.
    pub fn register(self, bp: &mut Blueprint) -> RegisteredConstructor {
        bp.register_constructor(self)
    }
}
