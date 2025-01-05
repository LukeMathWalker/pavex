use crate::blueprint::constructor::Constructor;
use crate::blueprint::linter::Lint;
use crate::blueprint::middleware::PostProcessingMiddleware;
use crate::blueprint::Blueprint;
use crate::f;

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
    /// The constructor for [`RequestCookies`].
    ///
    /// By default, it uses [`extract_request_cookies`].
    /// The error is handled by [`ExtractRequestCookiesError::into_response`].
    ///
    /// [`ExtractRequestCookiesError::into_response`]: super::errors::ExtractRequestCookiesError::into_response
    /// [`extract_request_cookies`]: super::extract_request_cookies
    /// [`RequestCookies`]: super::RequestCookies
    pub request_cookies: Option<Constructor>,
    /// The constructor for [`ResponseCookies`].
    ///
    /// By default, it uses [`ResponseCookies::new`].
    ///
    /// [`ResponseCookies::new`]: super::ResponseCookies::new
    /// [`ResponseCookies`]: super::ResponseCookies
    pub response_cookies: Option<Constructor>,
    /// The constructor for [`Processor`].
    ///
    /// By default, it uses [`Processor::from`]
    ///
    /// [`Processor`]: super::Processor
    /// [`Processor::from`]: super::Processor::from
    pub processor: Option<Constructor>,
    /// The constructor for [`ProcessorConfig`].
    ///
    /// By default, it's `None`.
    /// You can use [`with_default_processor_config`] to set it [`ProcessorConfig::default`].
    ///
    /// [`ProcessorConfig`]: super::ProcessorConfig
    /// [`ProcessorConfig::default`]: super::ProcessorConfig::default
    /// [`with_default_processor_config`]: CookieKit::with_default_processor_config
    pub processor_config: Option<Constructor>,
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
        let request_cookies = Constructor::request_scoped(f!(super::extract_request_cookies))
            .error_handler(f!(super::errors::ExtractRequestCookiesError::into_response))
            .ignore(Lint::Unused);
        let response_cookies =
            Constructor::request_scoped(f!(super::ResponseCookies::new)).ignore(Lint::Unused);
        let response_cookie_injector =
            PostProcessingMiddleware::new(f!(super::inject_response_cookies))
                .error_handler(f!(super::errors::InjectResponseCookiesError::into_response));
        let processor = Constructor::singleton(f!(<super::Processor as std::convert::From<
            super::ProcessorConfig,
        >>::from))
        .ignore(Lint::Unused);
        Self {
            request_cookies: Some(request_cookies),
            response_cookies: Some(response_cookies),
            response_cookie_injector: Some(response_cookie_injector),
            processor: Some(processor),
            processor_config: None,
        }
    }

    /// Set the [`ProcessorConfig`] constructor to [`ProcessorConfig::default`].
    ///
    /// [`ProcessorConfig`]: super::ProcessorConfig
    /// [`ProcessorConfig::default`]: super::ProcessorConfig::default
    pub fn with_default_processor_config(mut self) -> Self {
        let constructor = Constructor::singleton(f!(
            <super::ProcessorConfig as std::default::Default>::default
        ))
        .ignore(Lint::Unused);
        self.processor_config = Some(constructor);
        self
    }

    /// Register all the bundled constructors and middlewares with a [`Blueprint`].
    ///
    /// If a component is set to `None` it will not be registered.
    pub fn register(self, bp: &mut Blueprint) -> RegisteredCookieKit {
        if let Some(request_cookies) = self.request_cookies {
            request_cookies.register(bp);
        }
        if let Some(response_cookies) = self.response_cookies {
            response_cookies.register(bp);
        }
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
