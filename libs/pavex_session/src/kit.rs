use pavex::blueprint::Blueprint;
use pavex::blueprint::linter::Lint;
use pavex::blueprint::middleware::PostProcessingMiddleware;
use pavex::f;

use crate::middleware::FINALIZE_SESSION;

#[derive(Clone, Debug)]
#[non_exhaustive]
/// A collection of components required to work with sessions.
///
/// # Guide
///
/// Check out the [session installation](https://pavex.dev/guide/sessions/installation/)
/// section of Pavex's guide for a thorough introduction to sessions and how to
/// customize them.
///
/// # Example
///
/// ```rust
/// use pavex::blueprint::Blueprint;
/// use pavex::cookie::CookieKit;
/// use pavex_session::SessionKit;
///
/// let mut bp = Blueprint::new();
/// SessionKit::new().register(&mut bp);
/// // Sessions are built on top of cookies,
/// // so you need to set those up too.
/// // Order is important here!
/// CookieKit::new().register(&mut bp);
/// ```
pub struct SessionKit {
    /// A post-processing middleware to sync the session state with the session store
    /// and inject the session cookie into the outgoing response via the `Set-Cookie` header.
    ///
    /// By default, it's set to [`finalize_session`].
    /// The error is handled by [`FinalizeError::into_response`].
    ///
    /// [`FinalizeError::into_response`]: crate::errors::FinalizeError::into_response
    /// [`finalize_session`]: crate::middleware::finalize_session
    pub session_finalizer: Option<PostProcessingMiddleware>,
}

impl Default for SessionKit {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionKit {
    /// Create a new [`SessionKit`] with all the bundled constructors and middlewares.
    pub fn new() -> Self {
        Self {
            session_finalizer: Some(PostProcessingMiddleware::new(FINALIZE_SESSION)),
        }
    }

    #[doc(hidden)]
    #[deprecated(note = "This call is no longer necessary. \
        The session configuration will automatically use its default values if left unspecified.")]
    pub fn with_default_config(self) -> Self {
        self
    }

    /// Register all the bundled constructors and middlewares with a [`Blueprint`].
    ///
    /// If a component is set to `None` it will not be registered.
    pub fn register(self, bp: &mut Blueprint) -> RegisteredSessionKit {
        // Accessors for the fields on session config.
        bp.transient(f!(crate::SessionConfig::cookie_config))
            .ignore(Lint::Unused);
        bp.transient(f!(crate::SessionConfig::state_config))
            .ignore(Lint::Unused);

        if let Some(session_finalizer) = self.session_finalizer {
            session_finalizer.register(bp);
        }
        RegisteredSessionKit {}
    }
}

#[derive(Clone, Debug)]
#[non_exhaustive]
/// The type returned by [`SessionKit::register`].
pub struct RegisteredSessionKit {}
