use crate::blueprint::Blueprint;
use crate::blueprint::conversions::raw_identifiers2callable;
use crate::blueprint::middleware::pre::RegisteredPreProcessingMiddleware;
use crate::blueprint::raw::RawPreProcessingMiddleware;
use crate::blueprint::reflection::{RawIdentifiers, WithLocation};
use pavex_bp_schema::Callable;

/// A pre-processing middleware that has been configured but has not yet been registered with a [`Blueprint`].
///
/// # Guide
///
/// Check out [`Blueprint::pre_process`] for an introduction to pre_processing middlewares in Pavex.
#[derive(Clone, Debug)]
pub struct PreProcessingMiddleware {
    pub(in crate::blueprint) callable: Callable,
    pub(in crate::blueprint) error_handler: Option<Callable>,
}

impl PreProcessingMiddleware {
    /// Create a new (unregistered) pre_processing middleware.
    ///
    /// Check out the documentation of [`Blueprint::pre_process`] for more details
    /// on pre-processing middlewares.
    #[track_caller]
    pub fn new(m: RawPreProcessingMiddleware) -> Self {
        Self {
            callable: raw_identifiers2callable(m.coordinates),
            error_handler: None,
        }
    }

    /// Register an error handler for this middleware.
    ///
    /// Check out the documentation of [`RegisteredPreProcessingMiddleware::error_handler`] for more details.
    #[track_caller]
    pub fn error_handler(mut self, error_handler: WithLocation<RawIdentifiers>) -> Self {
        self.error_handler = Some(raw_identifiers2callable(error_handler));
        self
    }

    /// Register this middleware with a [`Blueprint`].
    ///
    /// Check out the documentation of [`Blueprint::pre_process`] for more details.
    pub fn register(self, bp: &mut Blueprint) -> RegisteredPreProcessingMiddleware {
        bp.register_pre_processing_middleware(self)
    }
}
