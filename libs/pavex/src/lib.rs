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

/// Define a constructor.
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

/// Define a request-scoped constructor.
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

/// Define a singleton constructor.
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

/// Define a transient constructor.
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

/// Define a wrapping middleware.
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
/// bp.wrap(TIMEOUT_ID);
/// # }
/// ```
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
