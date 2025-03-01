use pavex::blueprint::Blueprint;
use pavex::blueprint::config::ConfigType;
use pavex::blueprint::constructor::Constructor;
use pavex::blueprint::linter::Lint;
use pavex::blueprint::middleware::PostProcessingMiddleware;
use pavex::{f, t};

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
/// SessionKit::new()
///     .with_default_config()
///     .register(&mut bp);
/// // Sessions are built on top of cookies,
/// // so you need to set those up too.
/// // Order is important here!
/// CookieKit::new()
///     .with_default_processor_config()
///     .register(&mut bp);
/// ```
pub struct SessionKit {
    /// The constructor for [`Session`].
    ///
    /// By default, it uses [`Session::new`].
    ///
    /// [`Session`]: crate::Session
    /// [`Session::new`]: crate::Session::new
    pub session: Option<Constructor>,
    /// The constructor for [`IncomingSession`].
    ///
    /// By default, it uses [`IncomingSession::extract`].
    ///
    /// [`IncomingSession`]: crate::IncomingSession
    /// [`IncomingSession::extract`]: crate::IncomingSession::extract
    pub incoming_session: Option<Constructor>,
    /// Register [`SessionConfig`] as configuration.
    ///
    /// By default, it uses the `session` key.
    ///
    /// [`SessionConfig`]: crate::SessionConfig
    pub session_config: Option<ConfigType>,
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
        let session = Constructor::request_scoped(f!(crate::Session::new)).ignore(Lint::Unused);
        let incoming_session =
            Constructor::request_scoped(f!(crate::IncomingSession::extract)).ignore(Lint::Unused);
        let session_finalizer =
            PostProcessingMiddleware::new(f!(crate::middleware::finalize_session))
                .error_handler(f!(crate::errors::FinalizeError::into_response));
        let session_config =
            ConfigType::new("session", t!(crate::SessionConfig)).default_if_missing();
        Self {
            session: Some(session),
            incoming_session: Some(incoming_session),
            session_config: Some(session_config),
            session_finalizer: Some(session_finalizer),
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
        if let Some(session) = self.session {
            session.register(bp);
        }
        if let Some(incoming_session) = self.incoming_session {
            incoming_session.register(bp);
        }
        if let Some(session_config) = self.session_config {
            session_config.register(bp);
        }
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
