use crate::blueprint::Blueprint;
use crate::blueprint::config::ConfigType;
use crate::blueprint::constructor::Constructor;
use crate::blueprint::linter::Lint;
use crate::blueprint::middleware::PostProcessingMiddleware;
use crate::{f, t};

#[derive(Clone, Debug)]
#[non_exhaustive]
/// A collection of components required to work with request and response cookies.
///
/// # Guide
///
/// Check out the [cookie installation](https://pavex.dev/guide/cookies/installation/)
/// section of Pavex's guide for a thorough introduction to cookies and how to
/// customize them.
///
/// # Example
///
/// ```rust
/// use pavex::blueprint::Blueprint;
/// use pavex::cookie::CookieKit;
///
/// let mut bp = Blueprint::new();
/// let kit = CookieKit::new().register(&mut bp);
/// ```
pub struct CookieKit {
    /// The constructor for [`Processor`].
    ///
    /// By default, it uses [`Processor::from`]
    ///
    /// [`Processor`]: super::Processor
    /// [`Processor::from`]: super::Processor::from
    pub processor: Option<Constructor>,
    /// Register [`ProcessorConfig`] as a configuration type.
    ///
    /// By default, it uses `cookies` as its configuration key.
    ///
    /// [`ProcessorConfig`]: super::ProcessorConfig
    pub processor_config: Option<ConfigType>,
    /// A post-processing middleware to inject response cookies into the outgoing response
    /// via the `Set-Cookie` header.
    ///
    /// By default, it's set to [`inject_response_cookies`].
    /// The error is handled by [`InjectResponseCookiesError::into_response`].
    ///
    /// [`InjectResponseCookiesError::into_response`]: super::errors::InjectResponseCookiesError::into_response
    /// [`inject_response_cookies`]: super::inject_response_cookies
    pub response_cookie_injector: Option<PostProcessingMiddleware>,
}

impl Default for CookieKit {
    fn default() -> Self {
        Self::new()
    }
}

impl CookieKit {
    /// Create a new [`CookieKit`] with all the bundled constructors and middlewares.
    pub fn new() -> Self {
        let response_cookie_injector =
            PostProcessingMiddleware::new(super::components::INJECT_RESPONSE_COOKIES);
        let processor = Constructor::singleton(f!(<super::Processor as std::convert::From<
            super::ProcessorConfig,
        >>::from))
        .ignore(Lint::Unused);
        let processor_config =
            ConfigType::new("cookies", t!(super::ProcessorConfig)).default_if_missing();
        Self {
            response_cookie_injector: Some(response_cookie_injector),
            processor: Some(processor),
            processor_config: Some(processor_config),
        }
    }

    #[doc(hidden)]
    #[deprecated(note = "This call is no longer necessary. \
        The cookie processor configuration will automatically use its default values if left unspecified.")]
    pub fn with_default_processor_config(self) -> Self {
        self
    }

    /// Register all the bundled constructors and middlewares with a [`Blueprint`].
    ///
    /// If a component is set to `None` it will not be registered.
    pub fn register(self, bp: &mut Blueprint) -> RegisteredCookieKit {
        if let Some(response_cookie_injector) = self.response_cookie_injector {
            response_cookie_injector.register(bp);
        }
        if let Some(processor) = self.processor {
            processor.register(bp);
        }
        if let Some(processor_config) = self.processor_config {
            processor_config.register(bp);
        }
        RegisteredCookieKit {}
    }
}

#[derive(Clone, Debug)]
#[non_exhaustive]
/// The type returned by [`CookieKit::register`].
pub struct RegisteredCookieKit {}
