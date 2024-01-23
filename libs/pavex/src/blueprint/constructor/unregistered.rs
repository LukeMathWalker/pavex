use crate::blueprint::constructor::{CloningStrategy, Lifecycle, RegisteredConstructor};
use crate::blueprint::reflection::RawCallable;
use crate::blueprint::Blueprint;

/// A constructor that has not yet been registered with a [`Blueprint`].
///
/// It is primarily used by [`ApiKit`](crate::kit::ApiKit) to allow you to customize
/// (or disable!) the bundled constructors before registering them with a [`Blueprint`].
///
/// Check out the documentation of [`Blueprint::constructor`] for more details
/// on constructors in Pavex.
pub struct Constructor {
    callable: RawCallable,
    lifecycle: Lifecycle,
    cloning_strategy: Option<CloningStrategy>,
    error_handler: Option<RawCallable>,
}

impl Constructor {
    /// Create a new (unregistered) constructor.
    ///
    /// Check out the documentation of [`Blueprint::constructor`] for more details
    /// on constructors.
    pub fn new(callable: RawCallable, lifecycle: Lifecycle) -> Self {
        Self {
            callable,
            lifecycle,
            cloning_strategy: None,
            error_handler: None,
        }
    }

    #[track_caller]
    /// Register an error handler for this constructor.
    ///
    /// Check out the documentation of [`RegisteredConstructor::error_handler`] for more details.
    pub fn error_handler(mut self, error_handler: RawCallable) -> Self {
        self.error_handler = Some(error_handler);
        self
    }

    /// Set the cloning strategy for the output type returned by this constructor.
    ///
    /// Check out the documentation of [`RegisteredConstructor::cloning`] for more details.
    pub fn cloning(mut self, cloning_strategy: CloningStrategy) -> Self {
        self.cloning_strategy = Some(cloning_strategy);
        self
    }

    /// Register this constructor with a [`Blueprint`].
    ///
    /// Check out the documentation of [`Blueprint::constructor`] for more details.
    pub fn register(self, bp: &mut Blueprint) -> RegisteredConstructor {
        let mut c = bp.constructor(self.callable, self.lifecycle);
        if let Some(error_handler) = self.error_handler {
            c = c.error_handler(error_handler)
        }
        if let Some(cloning_strategy) = self.cloning_strategy {
            c = c.cloning(cloning_strategy)
        }
        c
    }
}
