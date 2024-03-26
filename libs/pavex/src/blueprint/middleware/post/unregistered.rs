use crate::blueprint::conversions::raw_callable2registered_callable;
use crate::blueprint::middleware::RegisteredPostProcessingMiddleware;
use crate::blueprint::reflection::RawCallable;
use crate::blueprint::Blueprint;
use pavex_bp_schema::Callable;

/// A post-processing middleware that has been configured
/// but has not yet been registered with a [`Blueprint`].
///
/// # Guide
///
/// Check out [`Blueprint::post_process`] for an introduction to post-processing
/// middlewares in Pavex.
///
/// # Use cases
///
/// [`PostProcessingMiddleware`] is primarily used by
/// [kits](https://pavex.dev/docs/guide/dependency_injection/core_concepts/kits)
/// to allow users to customize (or disable!)
/// the bundled middlewares **before** registering them with a [`Blueprint`].
#[derive(Clone, Debug)]
pub struct PostProcessingMiddleware {
    pub(in crate::blueprint) callable: Callable,
    pub(in crate::blueprint) error_handler: Option<Callable>,
}

impl PostProcessingMiddleware {
    /// Create a new (unregistered) post-processing middleware.
    ///
    /// Check out the documentation of [`Blueprint::post_process`] for more details
    /// on middleware.
    #[track_caller]
    pub fn new(callable: RawCallable) -> Self {
        Self {
            callable: raw_callable2registered_callable(callable),
            error_handler: None,
        }
    }

    /// Register an error handler for this middleware.
    ///
    /// Check out the documentation of [`RegisteredPostProcessingMiddleware::error_handler`] for more details.
    #[track_caller]
    pub fn error_handler(mut self, error_handler: RawCallable) -> Self {
        self.error_handler = Some(raw_callable2registered_callable(error_handler));
        self
    }

    /// Register this middleware with a [`Blueprint`].
    ///
    /// Check out the documentation of [`Blueprint::post_process`] for more details.
    pub fn register(self, bp: &mut Blueprint) -> RegisteredPostProcessingMiddleware {
        bp.register_post_processing_middleware(self)
    }
}
