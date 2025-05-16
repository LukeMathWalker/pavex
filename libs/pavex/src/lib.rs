//! # Pavex - API reference
//!
//! Welcome to the API reference for Pavex!
//!
//! The API reference is fairly low-level.\
//! If you want a high-level overview of Pavex, check out the [documentation](https://pavex.dev/docs/)
//! on Pavex's website.\
//! You'll also find [an installation guide](https://pavex.dev/docs/getting_started/) as well as a
//! [quickstart tutorial](https://pavex.dev/docs/getting_started/quickstart/)
//! to get you up and running with the framework in no time.

pub use error::error_::Error;

pub mod blueprint;
#[cfg(feature = "config")]
pub mod config;
pub mod connection;
#[cfg(feature = "cookie")]
pub mod cookie;
pub mod error;
pub mod http;
pub mod middleware;
pub mod request;
pub mod response;
pub mod router;
pub mod serialization;
#[cfg(feature = "server")]
pub mod server;
pub mod telemetry;
pub mod unit;
#[cfg(feature = "time")]
pub mod time {
    //! Utilities to work with dates, timestamps and datetimes.
    //!
    //! It's a re-export of the [`jiff@0.2`](https://docs.rs/jiff/0.2) crate.
    pub use jiff::*;
}

pub use pavex_macros::config;

/// Define a [constructor](https://pavex.dev/docs/guide/dependency_injection/constructors/).
///
/// Pavex will use the annotated function whenever it needs to create a new instance of
/// its output type.
///
/// # Imports
///
/// The annotated function must be imported via [`Blueprint::import`], otherwise it won't be considered
/// by Pavex.
///
/// # Guide
///
/// Check out the ["Dependency injection"](https://pavex.dev/docs/guide/dependency_injection)
/// section of Pavex's guide for a thorough introduction to dependency injection
/// in Pavex applications.
///
/// # Example
///
/// ```
/// use pavex::constructor;
///
/// pub struct MyType {
///     // [...]
/// }
///
/// impl MyType {
///     #[constructor(request_scoped)]
///     pub fn new() -> Self {
///         Self {
///             // [...]
///         }
///     }
/// }
/// ```
///
/// `MyType::new` will be called whenever a new instance of `MyType` is needed.
///
/// # Shortcuts
///
/// `#[constructor]` requires you to specify the lifetime of the instance it creates (i.e. `request_scoped` in the example above).
/// If you prefer a more concise syntax, you can use the lifetime-specific shortcuts:
///
/// ```
/// use pavex::request_scoped;
///
/// pub struct MyType {
///     // [...]
/// }
///
/// impl MyType {
///     // Equivalent to #[constructor(request_scoped)]
///     #[request_scoped]
///     pub fn new() -> Self {
///         Self {
///             // [...]
///         }
///     }
/// }
/// ```
///
/// [`#[request_scoped]`](request) is equivalent to `#[constructor(request_scoped)]`.\
/// [`#[singleton]`](singleton) is equivalent to `#[constructor(singleton)]`.\
/// [`#[transient]`](transient) is equivalent to `#[constructor(transient)]`.
///
/// [`Blueprint::import`]: crate::blueprint::Blueprint::import
pub use pavex_macros::constructor;

/// Define a [request-scoped constructor](https://pavex.dev/docs/guide/dependency_injection/constructors/).
///
/// Request-scoped constructors are invoked once per request to create a new instance
/// of their output type. The created instance is cached for the duration of the request
/// processing lifecycle.
///
/// # Imports
///
/// The annotated function must be imported via [`Blueprint::import`], otherwise it won't be considered
/// by Pavex.
///
/// # Guide
///
/// Check out the ["Dependency injection"](https://pavex.dev/docs/guide/dependency_injection)
/// section of Pavex's guide for a thorough introduction to dependency injection
/// in Pavex applications.
///
/// # Example
///
/// ```
/// use pavex::request_scoped;
///
/// pub struct MyType {
///     // [...]
/// }
///
/// impl MyType {
///     #[request_scoped]
///     pub fn new() -> Self {
///         Self {
///             // [...]
///         }
///     }
/// }
/// ```
///
/// `MyType::new` will be called once per request whenever a new instance of
/// `MyType` is needed.
///
/// [`Blueprint::import`]: crate::blueprint::Blueprint::import
pub use pavex_macros::request_scoped;

/// Define [a singleton constructor](https://pavex.dev/docs/guide/dependency_injection/constructors/).
///
/// Singleton constructors are invoked once (when the application starts up) to create
/// a new instance of their output type. The created instance is then shared across all
/// requests for the lifetime of the application.
///
/// # Imports
///
/// The annotated function must be imported via [`Blueprint::import`], otherwise it won't be considered
/// by Pavex.
///
/// # Guide
///
/// Check out the ["Dependency injection"](https://pavex.dev/docs/guide/dependency_injection)
/// section of Pavex's guide for a thorough introduction to dependency injection
/// in Pavex applications.
///
/// # Example
///
/// ```
/// use pavex::singleton;
///
/// pub struct MySharedResource {
///     // [...]
/// }
///
/// impl MySharedResource {
///     #[singleton]
///     pub fn new() -> Self {
///         Self {
///             // [...]
///         }
///     }
/// }
/// ```
///
/// `MySharedResource::new` will be called once at application startup, and the resulting
/// instance will be shared across all requests.
///
/// [`Blueprint::import`]: crate::blueprint::Blueprint::import
pub use pavex_macros::singleton;

/// Define a [transient constructor](https://pavex.dev/docs/guide/dependency_injection/constructors/).
///
/// Transient constructors are invoked each time a new instance of their output type
/// is needed, even within the same request. The created instances are not cached.
///
/// # Imports
///
/// The annotated function must be imported via [`Blueprint::import`], otherwise it won't be considered
/// by Pavex.
///
/// # Guide
///
/// Check out the ["Dependency injection"](https://pavex.dev/docs/guide/dependency_injection)
/// section of Pavex's guide for a thorough introduction to dependency injection
/// in Pavex applications.
///
/// # Example
///
/// ```
/// use pavex::transient;
///
/// pub struct MyType {
///     // [...]
/// }
///
/// impl MyType {
///     #[transient]
///     pub fn new() -> Self {
///         Self {
///             // [...]
///         }
///     }
/// }
/// ```
///
/// `MyType::new` will be called each time a new instance of `MyType` is needed,
/// even within the same request.
///
/// [`Blueprint::import`]: crate::blueprint::Blueprint::import
pub use pavex_macros::transient;

/// Define a [wrapping middleware](https://pavex.dev/docs/guide/middleware/wrapping/).
///
/// # Example
///
/// A middleware that applies a timeout to all incoming requests:
///
/// ```rust
/// use pavex::middleware::Next;
/// use pavex::response::Response;
/// use tokio::time::error::Elapsed;
///
/// #[pavex::wrap]
/// pub async fn timeout<C>(next: Next<C>) -> Result<Response, Elapsed>
/// where
///     C: IntoFuture<Output = Response>,
/// {
///     let max_duration = std::time::Duration::from_secs(20);
///     tokio::time::timeout(max_duration, next.into_future()).await
/// }
/// ```
///
/// # Guide
///
/// Check out the ["Middlewares"](https://pavex.dev/docs/guide/middleware/)
/// section of Pavex's guide for a thorough introduction to middlewares
/// in Pavex applications.
///
/// # Registration
///
/// You must invoke [`Blueprint::wrap`] to register the newly-defined middleware
/// with your [`Blueprint`].
/// `#[pavex::wrap]` generates a constant that can be used to refer to
/// the newly-defined middleware when interacting with your [`Blueprint`]:
///
/// ```rust
/// use pavex::blueprint::Blueprint;
///
/// # use pavex::middleware::Next;
/// # use pavex::response::Response;
/// # use tokio::time::error::Elapsed;
/// # #[pavex::wrap]
/// # pub async fn timeout<C>(next: Next<C>) -> Result<Response, Elapsed>
/// # where
/// #     C: IntoFuture<Output = Response>,
/// # {
/// #     let max_duration = std::time::Duration::from_secs(20);
/// #     tokio::time::timeout(max_duration, next.into_future()).await
/// # }
/// #
/// # fn main() {
/// let mut bp = Blueprint::new();
/// // The generated constant, by default, is named `<fn_name>_ID`,
/// // with `<fn_name>` converted to constant casing.
/// bp.wrap(TIMEOUT);
/// # }
/// ```
///
/// ## Customize the constant name
///
/// You can choose to customize the name of the generated constant via the `id`
/// macro argument:
///
/// ```rust
/// use pavex::blueprint::Blueprint;
/// use pavex::middleware::Next;
/// use pavex::response::Response;
/// use tokio::time::error::Elapsed;
///
/// // Custom id name 👇
/// #[pavex::wrap(id = "MY_TIMEOUT")]
/// pub async fn timeout<C>(next: Next<C>) -> Result<Response, Elapsed>
/// where
///     C: IntoFuture<Output = Response>,
/// {
///     // [...]
///     # let max_duration = std::time::Duration::from_secs(20);
///     # tokio::time::timeout(max_duration, next.into_future()).await
/// }
///
/// # fn main() {
/// let mut bp = Blueprint::new();
/// // Later used to register the middleware.
/// //          👇
/// bp.wrap(MY_TIMEOUT);
/// # }
/// ```
///
/// [`Blueprint::wrap`]: crate::blueprint::Blueprint::wrap
/// [`Blueprint`]: crate::blueprint::Blueprint
pub use pavex_macros::wrap;

/// Define a [pre-processing middleware](https://pavex.dev/docs/guide/middleware/pre_processing/).
///
/// # Example
///
/// Redirect requests to paths that end with a trailing `/` to
/// to the equivalent path without the trailing `/`:
///
/// ```rust
/// use pavex::http::{HeaderValue, header::LOCATION};
/// use pavex::middleware::Processing;
/// use pavex::request::RequestHead;
/// use pavex::response::Response;
///
/// #[pavex::pre_process]
/// pub fn redirect_to_normalized(head: &RequestHead) -> Processing {
///     let Some(normalized_path) = head.target.path().strip_suffix('/') else {
///         // No need to redirect, we continue processing the request.
///         return Processing::Continue;
///     };
///     let location = HeaderValue::from_str(normalized_path).unwrap();
///     let redirect = Response::temporary_redirect().insert_header(LOCATION, location);
///     // Short-circuit the request processing pipeline and return a redirect response
///     Processing::EarlyReturn(redirect)
/// }
/// ```
///
/// # Guide
///
/// Check out the ["Middlewares"](https://pavex.dev/docs/guide/middleware/)
/// section of Pavex's guide for a thorough introduction to middlewares
/// in Pavex applications.
///
/// # Registration
///
/// You must invoke [`Blueprint::pre_process`] to register the newly-defined middleware
/// with your [`Blueprint`].
/// `#[pavex::pre_process]` generates a constant that can be used to refer to
/// the newly-defined middleware when interacting with your [`Blueprint`]:
///
/// ```rust
/// use pavex::blueprint::Blueprint;
///
/// # use pavex::middleware::Processing;
/// # #[pavex::pre_process]
/// # pub fn redirect_to_normalized() -> Processing {
/// #     Processing::Continue
/// # }
/// # fn main() {
/// let mut bp = Blueprint::new();
/// // The generated constant, by default, is named `<fn_name>`,
/// // with `<fn_name>` converted to constant casing.
/// bp.pre_process(REDIRECT_TO_NORMALIZED);
/// # }
/// ```
///
/// ## Customize the constant name
///
/// You can choose to customize the name of the generated constant via the `id`
/// macro argument:
///
/// ```rust
/// use pavex::blueprint::Blueprint;
/// use pavex::middleware::Processing;
/// use pavex::request::RequestHead;
///
/// //               Custom id name 👇
/// #[pavex::pre_process(id = "TO_NORMALIZED")]
/// pub fn redirect_to_normalized(head: &RequestHead) -> Processing {
///     // [...]
///     # Processing::Continue
/// }
/// # fn main() {
/// let mut bp = Blueprint::new();
/// // Later used to register the middleware.
/// //                👇
/// bp.pre_process(TO_NORMALIZED);
/// # }
/// ```
///
/// [`Blueprint::pre_process`]: crate::blueprint::Blueprint::pre_process
/// [`Blueprint`]: crate::blueprint::Blueprint
pub use pavex_macros::pre_process;

/// Define a [post-processing middleware](https://pavex.dev/docs/guide/middleware/post_processing/).
///
/// # Example
///
/// Log the status code of the HTTP response returned to the caller:
///
/// ```rust
/// use pavex::response::Response;
/// use pavex_tracing::{
///     RootSpan,
///     fields::{http_response_status_code, HTTP_RESPONSE_STATUS_CODE}
/// };
///
/// #[pavex::post_process]
/// pub fn response_logger(response: Response, root_span: &RootSpan) -> Response
/// {
///     root_span.record(
///         HTTP_RESPONSE_STATUS_CODE,
///         http_response_status_code(&response),
///     );
///     response
/// }
/// ```
///
/// # Guide
///
/// Check out the ["Middlewares"](https://pavex.dev/docs/guide/middleware/)
/// section of Pavex's guide for a thorough introduction to middlewares
/// in Pavex applications.
///
/// # Registration
///
/// You must invoke [`Blueprint::post_process`] to register the newly-defined middleware
/// with your [`Blueprint`].
/// `#[pavex::post_process]` generates a constant that can be used to refer to
/// the newly-defined middleware when interacting with your [`Blueprint`]:
///
/// ```rust
/// use pavex::blueprint::Blueprint;
///
/// # use pavex::response::Response;
/// # use pavex_tracing::RootSpan;
/// # #[pavex::post_process]
/// # pub fn response_logger(response: Response, root_span: &RootSpan) -> Response
/// # {
/// #     todo!()
/// # }
/// # fn main() {
/// let mut bp = Blueprint::new();
/// // The generated constant, by default, is named `<fn_name>`,
/// // with `<fn_name>` converted to constant casing.
/// bp.pre_process(RESPONSE_LOGGER);
/// # }
/// ```
///
/// ## Customize the constant name
///
/// You can choose to customize the name of the generated constant via the `id`
/// macro argument:
///
/// ```rust
/// use pavex::blueprint::Blueprint;
/// use pavex::response::Response;
/// use pavex_tracing::RootSpan;
///
/// //               Custom id name 👇
/// #[pavex::post_process(id = "LOG_RESPONSE")]
/// pub fn response_logger(response: Response, root_span: &RootSpan) -> Response
/// {
///     // [..]
///     # todo!()
/// }
/// # fn main() {
/// let mut bp = Blueprint::new();
/// // Later used to register the middleware.
/// //                👇
/// bp.post_process(LOG_RESPONSE);
/// # }
/// ```
///
/// [`Blueprint::post_process`]: crate::blueprint::Blueprint::post_process
/// [`Blueprint`]: crate::blueprint::Blueprint
pub use pavex_macros::post_process;
