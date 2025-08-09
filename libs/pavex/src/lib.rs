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

pub use blueprint::blueprint::Blueprint;
pub use response::{into_response::IntoResponse, response_::Response};

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

/// Define a [prebuilt type](https://pavex.dev/docs/guide/dependency_injection/prebuilt_types/).
///
/// Prebuilt types are constructed outside of Pavex's dependency injection framework
/// but can still be injected as dependencies into your components.
///
/// # Example
///
/// ```rust
/// use pavex::prebuilt;
///
/// #[prebuilt]
/// #[derive(Debug)]
/// pub struct DatabaseConnectionPool {
///     // Connection pool initialized elsewhere
/// }
/// ```
///
/// # Guide
///
/// Check out the ["Dependency injection"](https://pavex.dev/docs/guide/dependency_injection)
/// section of Pavex's guide for a thorough introduction to dependency injection
/// in Pavex applications.
///
/// # Registration
///
/// You can register a prebuilt type with your application in two ways:
/// - Use [`Blueprint::prebuilt`] to register a single prebuilt type
/// - Use [`Blueprint::import`] to import multiple prebuilt types in bulk
///
/// The `#[prebuilt]` macro [generates a constant](#id) that you can use to refer to a specific
/// prebuilt type when invoking [`Blueprint::prebuilt`].
///
/// # Arguments
///
/// The sections below provide an exhaustive list of all the arguments and flags supported by the [`prebuilt`][macro@prebuilt] macro:
///
/// | Name                                          | Kind     | Required |
/// |-----------------------------------------------|----------|----------|
/// | [`id`](#id)                                   | Argument | No       |
/// | [`clone_if_necessary`](#clone_if_necessary)   | Flag     | No       |
/// | [`never_clone`](#never_clone)                 | Flag     | No       |
/// | [`allow`](#allow)                             | Argument | No       |
///
/// ## `id`
///
/// When using [`Blueprint::prebuilt`], Pavex generates a constant named after your type
/// (converted to UPPER_SNAKE_CASE) that you use to refer to the prebuilt type.
///
/// The `id` argument allows you to customize the name of the generated constant.
///
/// ### Example
///
/// ```rust
/// use pavex::{prebuilt, Blueprint};
///
/// #[prebuilt(id = "DB_POOL")]
/// //         👆 Custom identifier
/// #[derive(Debug)]
/// pub struct DatabaseConnectionPool {
///     // [...]
/// }
///
/// # fn main() {
/// let mut bp = Blueprint::new();
/// // Use the custom identifier when registering
/// bp.prebuilt(DB_POOL);
/// # }
/// ```
///
/// ## `clone_if_necessary`
///
/// By default, Pavex will **not** clone prebuilt types. The `clone_if_necessary` flag
/// allows Pavex to invoke `.clone()` on the type if it helps satisfy Rust's borrow checker.
///
/// The prebuilt type must implement the `Clone` trait.
///
/// This flag is mutually exclusive with `never_clone`.
///
/// ### Example
///
/// ```rust
/// use pavex::prebuilt;
///
/// #[prebuilt(clone_if_necessary)]
/// //         👆 Allow cloning when needed
/// #[derive(Debug, Clone)]
/// pub struct DatabaseConnectionPool {
///     // [...]
/// }
/// ```
///
/// ## `never_clone`
///
/// The `never_clone` flag explicitly prevents Pavex from cloning the prebuilt type.
/// This is the default behavior for prebuilt types, so this flag is typically used
/// for clarity and explicitness.
///
/// This flag is mutually exclusive with `clone_if_necessary`.
///
/// ### Example
///
/// ```rust
/// use pavex::prebuilt;
///
/// #[prebuilt(never_clone)]
/// //         👆 Explicitly prevent cloning (default behavior)
/// #[derive(Debug)]
/// pub struct NonCloneableResource {
///     // Some resource that shouldn't be cloned
///     file_handle: std::fs::File,
/// }
/// ```
///
/// Pavex will report an error during the code-generation phase if cloning is required
/// but forbidden.
///
/// ## `allow`
///
/// The `allow` argument can be used to suppress specific warnings.
///
/// Currently, only one value is supported:
/// - `unused`: Suppress warnings if this prebuilt type is registered but never used
///
/// ### Example
///
/// ```rust
/// use pavex::prebuilt;
///
/// #[prebuilt(allow(unused))]
/// //         👆 Don't warn if unused
/// #[derive(Debug, Clone)]
/// pub struct DatabaseConnectionPool {
///     // [...]
/// }
/// ```
///
/// [`Blueprint::import`]: crate::Blueprint::import
/// [`Blueprint::prebuilt`]: crate::Blueprint::prebuilt
pub use pavex_macros::prebuilt;

/// Define a [configuration type](https://pavex.dev/docs/guide/configuration/).
///
/// # Example
///
/// ```rust
/// use pavex::config;
///
/// #[config(key = "pool")]
/// #[derive(serde::Deserialize, Debug, Clone)]
/// pub struct PoolConfig {
///     pub max_n_connections: u32,
///     pub min_n_connections: u32,
/// }
/// ```
///
/// # Guide
///
/// Check out the ["Configuration"](https://pavex.dev/docs/guide/configuration/)
/// section of Pavex's guide for a thorough introduction to configuration
/// in Pavex applications.
///
/// # Import
///
/// Use [`Blueprint::config`] or [`Blueprint::import`] to add the newly-defined configuration type
/// to your application.
///
/// # Arguments
///
/// The sections below provide an exhaustive list of all the arguments and flags supported by the [`config`][macro@config] macro:
///
/// | Name                                        | Kind     | Required |
/// |---------------------------------------------|----------|----------|
/// | [`key`](#key)                               | Argument | Yes      |
/// | [`default_if_missing`](#default_if_missing) | Flag     | No       |
/// | [`include_if_unused`](#include_if_unused)   | Flag     | No       |
/// | [`never_clone`](#never_clone)               | Flag     | No       |
///
/// ## `key`
///
/// The `config` macro expects a `key` argument.\
/// `key` will be used as the name of the corresponding field in the
/// [generated `ApplicationConfig` struct][`ApplicationConfig`].
///
/// The provided key must satisfy a few constraints:
///
/// - It can't be empty.
/// - It must start with a letter.
/// - It can only contain letters, numbers, and underscores.
/// - It must be unique within the application.
///
/// ### Example
///
/// ```rust
/// use pavex::config;
///
/// #[config(key = "pool")]
/// //       👆 The key for this configuration type.
/// #[derive(serde::Deserialize, Debug, Clone)]
/// pub struct PoolConfig {
///     pub max_n_connections: u32,
///     pub min_n_connections: u32,
/// }
/// ```
///
/// ## `default_if_missing`
///
/// By default, Pavex considers all configuration types to be **required**.
/// There **must** be a value for each configuration key in the source we're reading from—e.g. a
/// YAML configuration file. If a key is missing, Pavex will return an error.
///
/// The `default_if_missing` flag disables this behaviour.
/// Pavex will invoke [`Default::default()`][`std::default::Default::default`] to provide a default
/// value if the configuration key was omitted by the configuration source. No error will be raised.
///
/// `default_if_missing` can only be used on types that implement the [`Default`] trait.
///
/// ### Example
///
/// Consider the following configuration type:
///
/// ```rust
/// use pavex::config;
///
/// #[config(key = "pool", default_if_missing)]
/// //                     👆 The flag
/// #[derive(serde::Deserialize, Debug, Clone)]
/// pub struct PoolConfig {
///     pub max_n_connections: u32,
///     pub min_n_connections: u32,
/// }
///
/// impl Default for PoolConfig {
///     fn default() -> Self {
///         Self {
///             max_n_connections: 10,
///             min_n_connections: 2,
///         }
///     }
/// }
/// ```
///
/// The definition of `ApplicationConfig`, in the generated server SDK code, will look like this:
///
/// ```rust
/// # #[derive(Debug, Clone, Default, serde::Deserialize)]
/// # struct PoolConfig {}
/// #[derive(serde::Deserialize, Debug, Clone)]
/// pub struct ApplicationConfig {
///     #[serde(default)]
///     // 👆 Tell `serde` to use the default value
///     //    if the field is missing.
///     pub pool: PoolConfig,
///     // [..other config types..]
/// }
/// ```
///
///
/// Therefore, given this YAML configuration file as input,
///
/// ```yaml
/// tls:
///   enabled: true
/// # No `pool` entry!
/// ```
///
/// Pavex will initialize `PoolConfig` using its default values, rather than returning an error.
///
/// ## `include_if_unused`
///
/// By default, Pavex will prune unused configuration types from the
/// final [`ApplicationConfig` type][`ApplicationConfig`].
///
/// The `include_if_unused` flag disables this behaviour.
/// There will always be an entry in [`ApplicationConfig`] for configuration types
/// marked as `include_if_unused`, even if their value is never accessed by any constructor,
/// route, or middleware.
///
/// ### Example
///
/// Consider this configuration type:
///
/// ```rust
/// use pavex::config;
///
/// #[config(key = "pool", include_if_unused)]
/// //                     👆 The flag
/// #[derive(serde::Deserialize, Debug, Clone)]
/// pub struct PoolConfig {
///     pub max_n_connections: u32,
///     pub min_n_connections: u32,
/// }
/// ```
///
/// The resulting `ApplicationConfig` will *always* contain an entry for `PoolConfig`,
/// even if it's never used. In the generated server SDK code:
///
/// ```rust
/// # #[derive(Debug, Clone, serde::Deserialize)]
/// # struct PoolConfig {}
/// #[derive(Debug, Clone, serde::Deserialize)]
/// pub struct ApplicationConfig {
///     pub pool: PoolConfig,
///     // 👆 You'll always find this field.
///
///     // [..other config types..]
/// }
/// ```
///
/// ## `never_clone`
///
/// By default, Pavex will invoke `.clone()` on configuration types if it helps to satisfy
/// Rust's borrow checker.
///
/// The `never_clone` flag disables this behaviour:
///
/// ```rust
/// use pavex::config;
///
/// #[config(key = "pool", never_clone)]
/// //                     👆 The flag
/// #[derive(serde::Deserialize, Debug)]
/// pub struct PoolConfig {
///     pub max_n_connections: u32,
///     pub min_n_connections: u32,
/// }
/// ```
///
/// Pavex will report an error during the code-generation phase if cloning is required
/// but forbidden.
///
/// [`Blueprint::import`]: crate::Blueprint::import
/// [`Blueprint::config`]: crate::Blueprint::config
/// [`ApplicationConfig`]: https://pavex.dev/docs/guide/configuration/application_config/
pub use pavex_macros::config;

/// Define a [request-scoped constructor](https://pavex.dev/docs/guide/dependency_injection/constructors/).
///
/// Request-scoped constructors are invoked once per request to create a new instance
/// of their output type. The created instance is cached for the duration of the request
/// processing lifecycle.
///
/// Check out [`#[singleton]`](macro@crate::singleton) and [`#[transient]`](macro@crate::transient)
/// if you need to define constructors with different lifecycles.
///
/// # Example
///
/// ```rust
/// use pavex::methods;
///
/// pub struct RequestId(uuid::Uuid);
///
/// #[methods]
/// impl RequestId {
///     #[request_scoped]
///     pub fn new() -> Self {
///         Self(uuid::Uuid::new_v4())
///     }
/// }
/// ```
///
/// `RequestId::new` will be called once per request whenever a new instance of
/// `RequestId` is needed.
///
/// # Guide
///
/// Check out the ["Dependency injection"](https://pavex.dev/docs/guide/dependency_injection)
/// section of Pavex's guide for a thorough introduction to dependency injection
/// in Pavex applications.
///
/// # Registration
///
/// You can register a request-scoped constructor with your application in two ways:
/// - Use [`Blueprint::constructor`] to register a single constructor
/// - Use [`Blueprint::import`] to import multiple constructors in bulk
///
/// The `#[request_scoped]` macro [generates a constant](#id) that you can use to refer to the
/// constructor when invoking [`Blueprint::constructor`].
///
/// # Arguments
///
/// The sections below provide an exhaustive list of all the arguments and flags supported by the [`request_scoped`][macro@request_scoped] macro:
///
/// | Name                                        | Kind     | Required |
/// |---------------------------------------------|----------|----------|
/// | [`id`](#id)                                 | Argument | No       |
/// | [`clone_if_necessary`](#clone_if_necessary) | Flag     | No       |
/// | [`never_clone`](#never_clone)               | Flag     | No       |
/// | [`allow`](#allow)                           | Argument | No       |
///
/// ## `id`
///
/// By default, Pavex generates a constant named after your type and method
/// (converted to UPPER_SNAKE_CASE) that you use when registering the constructor.
///
/// The `id` argument allows you to customize the name of the generated constant.
///
/// ### Example
///
/// Using the default generated identifier:
///
/// ```rust
/// use pavex::{methods, Blueprint};
///
/// pub struct RequestLogger {
///     // [...]
/// }
///
/// #[methods]
/// impl RequestLogger {
///     #[request_scoped]
///     pub fn new() -> Self {
///         Self {
///             // [...]
///         }
///     }
/// }
///
/// # fn main() {
/// let mut bp = Blueprint::new();
/// // The generated constant is named `REQUEST_LOGGER_NEW`
/// bp.constructor(REQUEST_LOGGER_NEW);
/// # }
/// ```
///
/// Using a custom identifier:
///
/// ```rust
/// use pavex::{methods, Blueprint};
///
/// pub struct RequestLogger {
///     // [...]
/// }
///
/// #[methods]
/// impl RequestLogger {
///     #[request_scoped(id = "LOGGER_CONSTRUCTOR")]
///     //               👆 Custom identifier
///     pub fn new() -> Self {
///         Self {
///             // [...]
///         }
///     }
/// }
///
/// # fn main() {
/// let mut bp = Blueprint::new();
/// // Use the custom identifier when registering
/// bp.constructor(LOGGER_CONSTRUCTOR);
/// # }
/// ```
///
/// ## `clone_if_necessary`
///
/// By default, Pavex will **not** clone the output of request-scoped constructors. The `clone_if_necessary`
/// flag allows Pavex to invoke `.clone()` on the output if it helps satisfy Rust's borrow checker.
///
/// The constructed type must implement the `Clone` trait.
///
/// This flag is mutually exclusive with `never_clone`.
///
/// ### Example
///
/// ```rust
/// use pavex::methods;
/// use std::collections::HashMap;
///
/// #[derive(Clone)]
/// pub struct RequestConfig {
///     settings: HashMap<String, String>,
/// }
///
/// #[methods]
/// impl RequestConfig {
///     #[request_scoped(clone_if_necessary)]
///     //               👆 Allow cloning when needed
///     pub fn new() -> Self {
///         Self {
///             settings: HashMap::new(),
///         }
///     }
/// }
/// ```
///
/// ## `never_clone`
///
/// The `never_clone` flag explicitly prevents Pavex from cloning the output of this constructor.
/// This is the default behavior for request-scoped constructors, so this flag is typically used
/// for clarity and explicitness.
///
/// This flag is mutually exclusive with `clone_if_necessary`.
///
/// ### Example
///
/// ```rust
/// use pavex::methods;
///
/// pub struct ScratchPad {
///     // Contains non-cloneable resources
///     temp_file: std::fs::File,
/// }
///
/// #[methods]
/// impl ScratchPad {
///     #[request_scoped(never_clone)]
///     //               👆 Explicitly prevent cloning (default behavior)
///     pub fn new() -> std::io::Result<Self> {
///         Ok(Self {
///             temp_file: std::fs::File::create("request.tmp")?,
///         })
///     }
/// }
/// ```
///
/// Pavex will report an error during the code-generation phase if cloning is required
/// but forbidden.
///
/// ## `allow`
///
/// The `allow` argument can be used to suppress specific warnings.
///
/// Currently, only one value is supported:
/// - `unused`: Suppress warnings if this constructor is registered but never used
///
/// ### Example
///
/// ```rust
/// use pavex::methods;
///
/// pub struct DebugInfo {
///     // [...]
/// }
///
/// #[methods]
/// impl DebugInfo {
///     #[request_scoped(allow(unused))]
///     //               👆 Don't warn if unused
///     pub fn new() -> Self {
///         Self {
///             // [...]
///         }
///     }
/// }
/// ```
///
/// [`Blueprint::import`]: crate::Blueprint::import
/// [`Blueprint::constructor`]: crate::Blueprint::constructor
#[doc(alias = "constructor")]
pub use pavex_macros::request_scoped;

/// Define a [singleton constructor](https://pavex.dev/docs/guide/dependency_injection/constructors/).
///
/// Singleton constructors are invoked once (when the application starts up) to create
/// a new instance of their output type. The created instance is then shared across all
/// requests for the lifetime of the application.
///
/// Check out [`#[request_scoped]`](macro@crate::request_scoped) and [`#[transient]`](macro@crate::transient)
/// if you need to define constructors with different lifecycles.
///
/// # Example
///
/// ```rust
/// use pavex::methods;
///
/// pub struct DatabasePool {
///     // [...]
/// }
///
/// #[methods]
/// impl DatabasePool {
///     #[singleton]
///     pub fn new() -> Self {
///         Self {
///             // [...]
///         }
///     }
/// }
/// ```
///
/// `DatabasePool::new` will be called once at application startup, and the resulting
/// instance will be shared across all requests.
///
/// # Guide
///
/// Check out the ["Dependency injection"](https://pavex.dev/docs/guide/dependency_injection)
/// section of Pavex's guide for a thorough introduction to dependency injection
/// in Pavex applications.
///
/// # Registration
///
/// You can register a singleton constructor with your application in two ways:
/// - Use [`Blueprint::constructor`] to register a single constructor
/// - Use [`Blueprint::import`] to import multiple constructors in bulk
///
/// The `#[singleton]` macro [generates a constant](#id) that you can use to refer to the
/// constructor when invoking [`Blueprint::constructor`].
///
/// # Arguments
///
/// The sections below provide an exhaustive list of all the arguments and flags supported by the [`singleton`][macro@singleton] macro:
///
/// | Name                                        | Kind     | Required |
/// |---------------------------------------------|----------|----------|
/// | [`id`](#id)                                 | Argument | No       |
/// | [`clone_if_necessary`](#clone_if_necessary) | Flag     | No       |
/// | [`never_clone`](#never_clone)               | Flag     | No       |
/// | [`allow`](#allow)                           | Argument | No       |
///
/// ## `id`
///
/// By default, Pavex generates a constant named after your type and method
/// (converted to UPPER_SNAKE_CASE) that you use when registering the constructor.
///
/// The `id` argument allows you to customize the name of the generated constant.
///
/// ### Example
///
/// Using the default generated identifier:
///
/// ```rust
/// use pavex::{methods, Blueprint};
///
/// pub struct CacheManager {
///     // [...]
/// }
///
/// #[methods]
/// impl CacheManager {
///     #[singleton]
///     pub fn new() -> Self {
///         Self {
///             // [...]
///         }
///     }
/// }
///
/// # fn main() {
/// let mut bp = Blueprint::new();
/// // The generated constant is named `CACHE_MANAGER_NEW`
/// bp.constructor(CACHE_MANAGER_NEW);
/// # }
/// ```
///
/// Using a custom identifier:
///
/// ```rust
/// use pavex::{methods, Blueprint};
///
/// pub struct CacheManager {
///     // [...]
/// }
///
/// #[methods]
/// impl CacheManager {
///     #[singleton(id = "CACHE_CONSTRUCTOR")]
///     //           👆 Custom identifier
///     pub fn new() -> Self {
///         Self {
///             // [...]
///         }
///     }
/// }
///
/// # fn main() {
/// let mut bp = Blueprint::new();
/// // Use the custom identifier when registering
/// bp.constructor(CACHE_CONSTRUCTOR);
/// # }
/// ```
///
/// ## `clone_if_necessary`
///
/// By default, Pavex will **not** clone the output of singleton constructors. The `clone_if_necessary`
/// flag allows Pavex to invoke `.clone()` on the output if it helps satisfy Rust's borrow checker.
///
/// The constructed type must implement the `Clone` trait.
///
/// This flag is mutually exclusive with `never_clone`.
///
/// ### Example
///
/// ```rust
/// use pavex::methods;
/// use std::collections::HashMap;
///
/// #[derive(Clone)]
/// pub struct GlobalConfig {
///     settings: HashMap<String, String>,
/// }
///
/// #[methods]
/// impl GlobalConfig {
///     #[singleton(clone_if_necessary)]
///     //          👆 Allow cloning when needed
///     pub fn new() -> Self {
///         Self {
///             settings: HashMap::new(),
///         }
///     }
/// }
/// ```
///
/// ## `never_clone`
///
/// The `never_clone` flag explicitly prevents Pavex from cloning the output of this constructor.
/// This is the default behavior for singleton constructors, so this flag is typically used
/// for clarity and explicitness.
///
/// This flag is mutually exclusive with `clone_if_necessary`.
///
/// ### Example
///
/// ```rust
/// use pavex::methods;
///
/// pub struct LogSink {
///     // Contains non-cloneable resources
///     handle: std::fs::File,
/// }
///
/// #[methods]
/// impl LogSink {
///     #[singleton(never_clone)]
///     //          👆 Explicitly prevent cloning (default behavior)
///     pub fn new() -> std::io::Result<Self> {
///         Ok(Self {
///             handle: std::fs::File::create("global.log")?,
///         })
///     }
/// }
/// ```
///
/// Pavex will report an error during the code-generation phase if cloning is required
/// but forbidden.
///
/// ## `allow`
///
/// The `allow` argument can be used to suppress specific warnings.
///
/// Currently, only one value is supported:
/// - `unused`: Suppress warnings if this constructor is registered but never used
///
/// ### Example
///
/// ```rust
/// use pavex::methods;
///
/// pub struct MetricsCollector {
///     // [...]
/// }
///
/// #[methods]
/// impl MetricsCollector {
///     #[singleton(allow(unused))]
///     //          👆 Don't warn if unused
///     pub fn new() -> Self {
///         Self {
///             // [...]
///         }
///     }
/// }
/// ```
///
/// [`Blueprint::import`]: crate::Blueprint::import
/// [`Blueprint::constructor`]: crate::Blueprint::constructor
#[doc(alias = "constructor")]
pub use pavex_macros::singleton;

/// Define a [transient constructor](https://pavex.dev/docs/guide/dependency_injection/constructors/).
///
/// Transient constructors are invoked each time a new instance of their output type
/// is needed, even within the same request. The created instances are not cached.
///
/// Check out [`#[singleton]`](macro@crate::singleton) and [`#[request_scoped]`](macro@crate::request_scoped)
/// if you need to define constructors with different lifecycles.
///
/// # Example
///
/// ```rust
/// use pavex::methods;
///
/// pub struct TemporaryBuffer {
///     data: Vec<u8>,
/// }
///
/// #[methods]
/// impl TemporaryBuffer {
///     #[transient]
///     pub fn new() -> Self {
///         Self {
///             data: Vec::with_capacity(1024),
///         }
///     }
/// }
/// ```
///
/// `TemporaryBuffer::new` will be called each time a new instance of `TemporaryBuffer` is needed,
/// even within the same request.
///
/// # Guide
///
/// Check out the ["Dependency injection"](https://pavex.dev/docs/guide/dependency_injection)
/// section of Pavex's guide for a thorough introduction to dependency injection
/// in Pavex applications.
///
/// # Registration
///
/// You can register a transient constructor with your application in two ways:
/// - Use [`Blueprint::constructor`] to register a single constructor
/// - Use [`Blueprint::import`] to import multiple constructors in bulk
///
/// The `#[transient]` macro [generates a constant](#id) that you can use to refer to the
/// constructor when invoking [`Blueprint::constructor`].
///
/// # Arguments
///
/// The sections below provide an exhaustive list of all the arguments and flags supported by the [`transient`][macro@transient] macro:
///
/// | Name                                        | Kind     | Required |
/// |---------------------------------------------|----------|----------|
/// | [`id`](#id)                                 | Argument | No       |
/// | [`clone_if_necessary`](#clone_if_necessary) | Flag     | No       |
/// | [`never_clone`](#never_clone)               | Flag     | No       |
/// | [`allow`](#allow)                           | Argument | No       |
///
/// ## `id`
///
/// By default, Pavex generates a constant named after your type and method
/// (converted to UPPER_SNAKE_CASE) that you use when registering the constructor.
///
/// The `id` argument allows you to customize the name of the generated constant.
///
/// ### Example
///
/// Using the default generated identifier:
///
/// ```rust
/// use pavex::{methods, Blueprint};
///
/// pub struct TokenGenerator {
///     // [...]
/// }
///
/// #[methods]
/// impl TokenGenerator {
///     #[transient]
///     pub fn new() -> Self {
///         Self {
///             // [...]
///         }
///     }
/// }
///
/// # fn main() {
/// let mut bp = Blueprint::new();
/// // The generated constant is named `TOKEN_GENERATOR_NEW`
/// bp.constructor(TOKEN_GENERATOR_NEW);
/// # }
/// ```
///
/// Using a custom identifier:
///
/// ```rust
/// use pavex::{methods, Blueprint};
///
/// pub struct TokenGenerator {
///     // [...]
/// }
///
/// #[methods]
/// impl TokenGenerator {
///     #[transient(id = "TOKEN_CONSTRUCTOR")]
///     //           👆 Custom identifier
///     pub fn new() -> Self {
///         Self {
///             // [...]
///         }
///     }
/// }
///
/// # fn main() {
/// let mut bp = Blueprint::new();
/// // Use the custom identifier when registering
/// bp.constructor(TOKEN_CONSTRUCTOR);
/// # }
/// ```
///
/// ## `clone_if_necessary`
///
/// By default, Pavex will **not** clone the output of transient constructors. The `clone_if_necessary`
/// flag allows Pavex to invoke `.clone()` on the output if it helps satisfy Rust's borrow checker.
///
/// The constructed type must implement the `Clone` trait.
///
/// This flag is mutually exclusive with `never_clone`.
///
/// ### Example
///
/// ```rust
/// use pavex::{transient, methods};
/// use uuid::Uuid;
///
/// #[derive(Clone)]
/// pub struct InstanceId {
///     value: Uuid,
/// }
///
/// #[methods]
/// impl InstanceId {
///     #[transient(clone_if_necessary)]
///     //           👆 Allow cloning when needed
///     pub fn new() -> Self {
///         Self {
///             value: uuid::Uuid::new_v4(),
///         }
///     }
/// }
/// ```
///
/// ## `never_clone`
///
/// The `never_clone` flag explicitly prevents Pavex from cloning the output of this constructor.
/// This is the default behavior for transient constructors, so this flag is typically used
/// for clarity and explicitness.
///
/// This flag is mutually exclusive with `clone_if_necessary`.
///
/// ### Example
///
/// ```rust
/// use pavex::methods;
///
/// pub struct TempFile {
///     // Contains non-cloneable resources
///     handle: std::fs::File,
/// }
///
/// #[methods]
/// impl TempFile {
///     #[transient(never_clone)]
///     //           👆 Explicitly prevent cloning (default behavior)
///     pub fn new() -> std::io::Result<Self> {
///         Ok(Self {
///             handle: std::fs::File::create_new("temp.tmp")?,
///         })
///     }
/// }
/// ```
///
/// Pavex will report an error during the code-generation phase if cloning is required
/// but forbidden.
///
/// ## `allow`
///
/// The `allow` argument can be used to suppress specific warnings.
///
/// Currently, only one value is supported:
/// - `unused`: Suppress warnings if this constructor is registered but never used
///
/// ### Example
///
/// ```rust
/// use pavex::methods;
///
/// pub struct TraceId {
///     // [...]
/// }
///
/// #[methods]
/// impl TraceId {
///     #[transient(allow(unused))]
///     //           👆 Don't warn if unused
///     pub fn new() -> Self {
///         Self {
///             // [...]
///         }
///     }
/// }
/// ```
///
/// [`Blueprint::import`]: crate::Blueprint::import
/// [`Blueprint::constructor`]: crate::Blueprint::constructor
#[doc(alias = "constructor")]
pub use pavex_macros::transient;

/// Define a [wrapping middleware](https://pavex.dev/docs/guide/middleware/wrapping/).
///
/// # Example
///
/// A middleware that applies a timeout to all incoming requests:
///
/// ```rust
/// use pavex::{wrap, middleware::Next, Response};
/// use tokio::time::error::Elapsed;
///
/// #[wrap]
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
/// Use [`Blueprint::wrap`] to register the middleware with your [`Blueprint`].
///
/// The `#[wrap]` macro [generates a constant](#id) that you can use to refer to the
/// middleware when invoking [`Blueprint::wrap`].
///
/// # Arguments
///
/// The sections below provide an exhaustive list of all the arguments and flags supported by the [`wrap`][macro@wrap] macro:
///
/// | Name                | Kind     | Required |
/// |---------------------|----------|----------|
/// | [`id`](#id)         | Argument | No       |
///
/// ## `id`
///
/// By default, Pavex generates a constant named after your function
/// (converted to UPPER_SNAKE_CASE) that you use when registering the middleware.
///
/// The `id` argument allows you to customize the name of the generated constant.
///
/// ### Example
///
/// Using the default generated identifier:
///
/// ```rust
/// use pavex::{wrap, Blueprint, middleware::Next, Response};
/// use tokio::time::error::Elapsed;
///
/// #[wrap]
/// pub async fn timeout<C>(next: Next<C>) -> Result<Response, Elapsed>
/// where
///     C: IntoFuture<Output = Response>,
/// {
///     let max_duration = std::time::Duration::from_secs(20);
///     tokio::time::timeout(max_duration, next.into_future()).await
/// }
///
/// # fn main() {
/// let mut bp = Blueprint::new();
/// bp.wrap(TIMEOUT);
/// # }
/// ```
///
/// Using a custom identifier:
///
/// ```rust
/// use pavex::{wrap, Blueprint, middleware::Next, Response};
/// use tokio::time::error::Elapsed;
///
/// #[wrap(id = "MY_TIMEOUT")]
/// //     👆 Custom identifier
/// pub async fn timeout<C>(next: Next<C>) -> Result<Response, Elapsed>
/// where
///     C: IntoFuture<Output = Response>,
/// {
///     let max_duration = std::time::Duration::from_secs(20);
///     tokio::time::timeout(max_duration, next.into_future()).await
/// }
///
/// # fn main() {
/// let mut bp = Blueprint::new();
/// // Use the custom identifier when registering
/// bp.wrap(MY_TIMEOUT);
/// # }
/// ```
///
/// [`Blueprint::wrap`]: crate::Blueprint::wrap
/// [`Blueprint`]: crate::Blueprint
#[doc(alias = "middleware")]
pub use pavex_macros::wrap;

/// Define a [pre-processing middleware](https://pavex.dev/docs/guide/middleware/pre_processing/).
///
/// # Example
///
/// Redirect requests to paths that end with a trailing `/` to
/// the equivalent path without the trailing `/`:
///
/// ```rust
/// use pavex::{pre_process, http::{HeaderValue, header::LOCATION}};
/// use pavex::middleware::Processing;
/// use pavex::request::RequestHead;
/// use pavex::Response;
///
/// #[pre_process]
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
/// Use [`Blueprint::pre_process`] to register the middleware with your [`Blueprint`].
///
/// The `#[pre_process]` macro [generates a constant](#id) that you can use to refer to the
/// middleware when invoking [`Blueprint::pre_process`].
///
/// # Arguments
///
/// The sections below provide an exhaustive list of all the arguments and flags supported by the [`pre_process`][macro@pre_process] macro:
///
/// | Name                | Kind     | Required |
/// |---------------------|----------|----------|
/// | [`id`](#id)         | Argument | No       |
///
/// ## `id`
///
/// By default, Pavex generates a constant named after your function
/// (converted to UPPER_SNAKE_CASE) that you use when registering the middleware.
///
/// The `id` argument allows you to customize the name of the generated constant.
///
/// ### Example
///
/// Using the default generated identifier:
///
/// ```rust
/// use pavex::{pre_process, Blueprint, middleware::Processing};
///
/// #[pre_process]
/// pub fn redirect_to_normalized() -> Processing {
///     Processing::Continue
/// }
///
/// # fn main() {
/// let mut bp = Blueprint::new();
/// bp.pre_process(REDIRECT_TO_NORMALIZED);
/// # }
/// ```
///
/// Using a custom identifier:
///
/// ```rust
/// use pavex::{pre_process, Blueprint, middleware::Processing, request::RequestHead};
///
/// #[pre_process(id = "TO_NORMALIZED")]
/// //            👆 Custom identifier
/// pub fn redirect_to_normalized(head: &RequestHead) -> Processing {
///     // [...]
///     Processing::Continue
/// }
///
/// # fn main() {
/// let mut bp = Blueprint::new();
/// // Use the custom identifier when registering
/// bp.pre_process(TO_NORMALIZED);
/// # }
/// ```
///
/// [`Blueprint::pre_process`]: crate::Blueprint::pre_process
/// [`Blueprint`]: crate::Blueprint
#[doc(alias = "middleware")]
pub use pavex_macros::pre_process;

/// Define a [post-processing middleware](https://pavex.dev/docs/guide/middleware/post_processing/).
///
/// # Example
///
/// Log the status code of the HTTP response returned to the caller:
///
/// ```rust
/// use pavex::{post_process, Response};
/// use pavex_tracing::{
///     RootSpan,
///     fields::{http_response_status_code, HTTP_RESPONSE_STATUS_CODE}
/// };
///
/// #[post_process]
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
/// Use [`Blueprint::post_process`] to register the middleware with your [`Blueprint`].
///
/// The `#[post_process]` macro [generates a constant](#id) that you can use to refer to the
/// middleware when invoking [`Blueprint::post_process`].
///
/// # Arguments
///
/// The sections below provide an exhaustive list of all the arguments and flags supported by the [`post_process`][macro@post_process] macro:
///
/// | Name                | Kind     | Required |
/// |---------------------|----------|----------|
/// | [`id`](#id)         | Argument | No       |
///
/// ## `id`
///
/// By default, Pavex generates a constant named after your function
/// (converted to UPPER_SNAKE_CASE) that you use when registering the middleware.
///
/// The `id` argument allows you to customize the name of the generated constant.
///
/// ### Example
///
/// Using the default generated identifier:
///
/// ```rust
/// use pavex::{post_process, Blueprint, Response};
/// use pavex_tracing::RootSpan;
///
/// #[post_process]
/// pub fn response_logger(response: Response, root_span: &RootSpan) -> Response
/// {
///     // [...]
///     # todo!()
/// }
///
/// # fn main() {
/// let mut bp = Blueprint::new();
/// // The generated constant is named `RESPONSE_LOGGER`
/// bp.post_process(RESPONSE_LOGGER);
/// # }
/// ```
///
/// Using a custom identifier:
///
/// ```rust
/// use pavex::{post_process, Blueprint, Response};
/// use pavex_tracing::RootSpan;
///
/// #[post_process(id = "LOG_RESPONSE")]
/// //             👆 Custom identifier
/// pub fn response_logger(response: Response, root_span: &RootSpan) -> Response
/// {
///     // [...]
///     # todo!()
/// }
///
/// # fn main() {
/// let mut bp = Blueprint::new();
/// // Use the custom identifier when registering
/// bp.post_process(LOG_RESPONSE);
/// # }
/// ```
///
/// [`Blueprint::post_process`]: crate::Blueprint::post_process
/// [`Blueprint`]: crate::Blueprint
#[doc(alias = "middleware")]
pub use pavex_macros::post_process;

/// Define an [error handler](https://pavex.dev/docs/guide/errors/error_handlers/).
///
/// Error handlers are invoked whenever an error occurs during request processing,
/// allowing you to transform errors into HTTP responses.
///
/// # Example
///
/// ```rust
/// use pavex::error_handler;
/// use pavex::Response;
/// # struct AuthError;
///
/// #[error_handler]
/// pub fn handle_auth_error(e: &AuthError) -> Response {
///     // Transform the error into an HTTP response
///     Response::unauthorized()
/// }
/// ```
///
/// # Guide
///
/// Check out the [\"Error handling\"](https://pavex.dev/docs/guide/errors)
/// section of Pavex's guide for a thorough introduction to error handling
/// in Pavex applications.
///
/// # Registration
///
/// You can register an error handler with your application in two ways:
/// - Use [`Blueprint::error_handler`] to register a single error handler
/// - Use [`Blueprint::import`] to import multiple error handlers in bulk
///
/// The `#[error_handler]` macro [generates a constant](#id) that you can use to refer to the
/// error handler when invoking [`Blueprint::error_handler`].
///
/// # Arguments
///
/// The sections below provide an exhaustive list of all the arguments and helper attributes supported by the [`error_handler`][macro@error_handler] macro:
///
/// | Name                          | Kind             | Required  |
/// |-------------------------------|------------------|-----------|
/// | [`px(error_ref)`](#error_ref) | Helper attribute | Sometimes |
/// | [`id`](#id)                   | Argument         | No        |
/// | [`default`](#default)         | Argument         | No        |
///
/// ## `error_ref`
///
/// One of the input parameters for an error handler must be a reference to the error type that's being handled.
///
/// Pavex infers the error type automatically if the error handler has a single input parameter.\
/// If there are multiple input parameters, you must annotate the error reference with `#[px(error_ref)]`.
///
/// ### Example: single input parameter
///
/// ```rust
/// use pavex::methods;
/// use pavex::Response;
///
/// pub struct AuthError { /* */ };
///
/// #[methods]
/// impl AuthError {
///     #[error_handler]
///     // A single input parameter, no need for additional annotations
///     // Pavex will infer that `&self` is the error reference
///     pub fn to_response(&self) -> Response {
///         // [...]
///         # Response::ok()
///     }
/// }
/// ```
///
/// ### Example: multiple input parameters
///
/// ```rust
/// use pavex::methods;
/// use pavex::Response;
///
/// pub struct AuthError { /* */ };
/// # pub struct OrgId;
///
/// #[methods]
/// impl AuthError {
///     #[error_handler]
///     pub fn to_response(
///         #[px(error_ref)] &self,
///         // 👆
///         // Multiple input parameters, you must mark the error reference
///         organization_id: OrgId
///     ) -> Response {
///         // [...]
///         # Response::ok()
///     }
/// }
/// ```
///
/// ## `id`
///
/// When using [`Blueprint::error_handler`], Pavex generates a constant named after your function
/// (converted to UPPER_SNAKE_CASE) that you can use to refer to the error handler
/// when invoking [`Blueprint::error_handler`].
///
/// The `id` argument allows you to customize the name of the generated constant.
///
/// ### Example
///
/// Using the default identifier:
///
/// ```rust
/// use pavex::{error_handler, Blueprint};
/// use pavex::Response;
/// # struct AuthError;
///
/// #[error_handler]
/// pub fn handle_auth_error(e: &AuthError) -> Response {
///     // [...]
///     # todo!()
/// }
///
/// let mut bp = Blueprint::new();
/// bp.error_handler(HANDLE_AUTH_ERROR);
/// ```
///
/// Using a custom identifier:
///
/// ```rust
/// use pavex::{error_handler, Blueprint};
/// use pavex::Response;
///
/// #[error_handler(id = "AUTH_HANDLER")]
/// //              👆 Custom identifier
/// pub fn handle_auth_error(e: &AuthError) -> Response {
///     // [...]
///     # todo!()
/// }
///
/// let mut bp = Blueprint::new();
/// bp.error_handler(AUTH_HANDLER);
/// # struct AuthError;
/// ```
///
/// ## `default`
///
/// The `default` argument determines whether this error handler should be used as the
/// default handler for the error type whenever an error of the matching type is emitted.
///
/// By default, error handlers are considered the default handler for their error type (`default = true`).
///
/// ### Example
///
/// ```rust
/// use pavex::{error_handler, get, Blueprint};
/// use pavex::Response;
///
/// #[error_handler]
/// // This is the default handler for `AuthError`s
/// pub fn handle_auth_error(e: &AuthError) -> Response {
///     # todo!()
/// }
///
/// #[error_handler(default = false)]
/// //              👆 Not the default handler
/// pub fn handle_auth_error_admin(e: &AuthError) -> Response {
///     # todo!()
/// }
///
/// #[get(path = "/admin")]
/// pub fn admin_route() -> Result<Response, AuthError> {
///     // Admin-specific logic
///     # todo!()
/// }
///
/// let mut bp = Blueprint::new();
/// // Register the default error handler
/// bp.error_handler(HANDLE_AUTH_ERROR);
/// // Specify a different error handler for the admin route
/// bp.route(ADMIN_ROUTE).error_handler(HANDLE_AUTH_ERROR_ADMIN);
/// # struct AuthError;
/// ```
pub use pavex_macros::error_handler;

/// Define an [error observer](https://pavex.dev/docs/guide/errors/error_observers/).
///
/// Error observers are invoked whenever an error occurs during request processing,
/// allowing you to log errors, send them to monitoring services, or perform other
/// observability tasks.
///
/// # Example
///
/// Log errors that occur during request processing:
///
/// ```rust
/// use pavex::error_observer;
/// use tracing_log_error::log_error;
///
/// #[error_observer]
/// pub async fn error_logger(e: &pavex::Error) {
///     log_error!(e)
/// }
/// ```
///
/// # Guide
///
/// Check out the ["Errors"](https://pavex.dev/docs/guide/errors/)
/// section of Pavex's guide for a thorough introduction to errors
/// in Pavex applications.
///
/// # Registration
///
/// Use [`Blueprint::error_observer`] to register the error observer with your [`Blueprint`].
///
/// The `#[error_observer]` macro [generates a constant](#id) that you can use to refer to the
/// observer when invoking [`Blueprint::error_observer`].
///
/// # Arguments
///
/// The sections below provide an exhaustive list of all the arguments and flags supported by the [`error_observer`][macro@error_observer] macro:
///
/// | Name                | Kind     | Required |
/// |---------------------|----------|----------|
/// | [`id`](#id)         | Argument | No       |
///
/// ## `id`
///
/// By default, Pavex generates a constant named after your function
/// (converted to UPPER_SNAKE_CASE) that you use when registering the error observer.
///
/// The `id` argument allows you to customize the name of the generated constant.
///
/// ### Example
///
/// Using the default generated identifier:
///
/// ```rust
/// use pavex::{error_observer, Blueprint};
///
/// #[error_observer]
/// pub async fn error_logger(e: &pavex::Error) {
///     // Log the error
/// }
///
/// # fn main() {
/// let mut bp = Blueprint::new();
/// // The generated constant is named `ERROR_LOGGER`
/// bp.error_observer(ERROR_LOGGER);
/// # }
/// ```
///
/// Using a custom identifier:
///
/// ```rust
/// use pavex::{error_observer, Blueprint};
///
/// #[error_observer(id = "MY_ERROR_LOGGER")]
/// //               👆 Custom identifier
/// pub async fn error_logger(e: &pavex::Error) {
///     // Log the error
/// }
///
/// # fn main() {
/// let mut bp = Blueprint::new();
/// // Use the custom identifier when registering
/// bp.error_observer(MY_ERROR_LOGGER);
/// # }
/// ```
///
/// [`Blueprint::error_observer`]: crate::Blueprint::error_observer
/// [`Blueprint`]: crate::Blueprint
pub use pavex_macros::error_observer;

macro_rules! http_method_macro_doc {
    ($method:literal, $macro_name:ident, $example_fn:ident, $constant_name:ident, $example_path:literal, $example_comment:literal) => {
        #[doc = concat!("Define a [route](https://pavex.dev/docs/guide/routing/) for ", $method, " requests to a given path.")]
        ///
        #[doc = concat!("This is a shorthand for [`#[route(method = \"", $method, "\", path = \"...\")]`](macro@route).")]
        ///
        /// # Example
        ///
        /// ```rust
        #[doc = concat!("use pavex::{", stringify!($macro_name), ", Response};")]
        ///
        #[doc = concat!("#[", stringify!($macro_name), "(path = \"", $example_path, "\")]")]
        #[doc = concat!("pub async fn ", stringify!($example_fn), "(/* */) -> Response {")]
        #[doc = concat!("    // ", $example_comment)]
        ///     // [...]
        ///     # Response::ok()
        /// }
        /// ```
        ///
        /// # Guide
        ///
        /// Check out the ["Routing"](https://pavex.dev/docs/guide/routing/)
        /// section of Pavex's guide for a thorough introduction to routing
        /// in Pavex applications.
        ///
        /// # Registration
        ///
        #[doc = concat!("You can register a ", $method, " route with your application in two ways:")]
        /// - Use [`Blueprint::route`] to register a single route
        /// - Use [`Blueprint::import`] to import multiple routes in bulk
        ///
        #[doc = concat!("The `#[", stringify!($macro_name), "]` macro [generates a constant](#id) that you can use to refer to the")]
        /// route when invoking [`Blueprint::route`].
        ///
        /// # Arguments
        ///
        #[doc = concat!("The sections below provide an exhaustive list of all the arguments supported by the [`", stringify!($macro_name), "`][macro@", stringify!($macro_name), "] macro:")]
        ///
        /// | Name                              | Kind     | Required |
        /// |-----------------------------------|----------|----------|
        /// | [`path`](#path)                   | Argument | Yes      |
        /// | [`id`](#id)                       | Argument | No       |
        ///
        /// ## `path`
        ///
        /// The `path` argument specifies the URL path pattern that this route will match.
        ///
        /// The path can contain dynamic parameter placeholders in the format `{parameter_name}`.
        /// Check out the [guide on parameter extraction](https://pavex.dev/docs/guide/routing/path_patterns/#path-parameters)
        /// for a detailed explanation.
        ///
        /// ### Example
        ///
        /// ```rust
        #[doc = concat!("use pavex::{", stringify!($macro_name), ", Response};")]
        ///
        #[doc = concat!("#[", stringify!($macro_name), "(path = \"", $example_path, "\")]")]
        /// //           👆 Path with parameter
        #[doc = concat!("pub async fn ", stringify!($example_fn), "(/* */) -> Response {")]
        ///     // [...]
        ///     # Response::ok()
        /// }
        /// ```
        ///
        /// ## `id`
        ///
        /// By default, Pavex generates a constant named after your function
        /// (converted to UPPER_SNAKE_CASE) that you can use when invoking [`Blueprint::route`].
        ///
        /// The `id` argument allows you to customize the name of the generated constant.
        ///
        /// ### Example
        ///
        /// Using the default generated identifier:
        ///
        /// ```rust
        #[doc = concat!("use pavex::{", stringify!($macro_name), ", Response, Blueprint};")]
        ///
        #[doc = concat!("#[", stringify!($macro_name), "(path = \"", $example_path, "\")]")]
        #[doc = concat!("pub async fn ", stringify!($example_fn), "(/* */) -> Response {")]
        ///     // [...]
        ///     # Response::ok()
        /// }
        ///
        /// # fn main() {
        /// let mut bp = Blueprint::new();
        #[doc = concat!("// The generated constant is named `", stringify!($constant_name), "`")]
        #[doc = concat!("bp.route(", stringify!($constant_name), ");")]
        /// # }
        /// ```
        ///
        /// Using a custom identifier:
        ///
        /// ```rust
        #[doc = concat!("use pavex::{", stringify!($macro_name), ", Response, Blueprint};")]
        ///
        #[doc = concat!("#[", stringify!($macro_name), "(path = \"", $example_path, "\", id = \"CUSTOM_ROUTE\")]")]
        /// //                           👆 Custom identifier
        #[doc = concat!("pub async fn ", stringify!($example_fn), "(id: u32) -> Response {")]
        ///     // [...]
        ///     # Response::ok()
        /// }
        ///
        /// # fn main() {
        /// let mut bp = Blueprint::new();
        /// // Use the custom identifier when registering
        /// bp.route(CUSTOM_ROUTE);
        /// # }
        /// ```
        ///
        /// [`Blueprint::import`]: crate::Blueprint::import
        /// [`Blueprint::route`]: crate::Blueprint::route
        #[doc(alias = "route")]
        #[doc(alias = "request_handler")]
        pub use pavex_macros::$macro_name;
    };
}

http_method_macro_doc!(
    "DELETE",
    delete,
    delete_user,
    DELETE_USER,
    "/users/{id}",
    "Delete user logic"
);
http_method_macro_doc!(
    "GET",
    get,
    get_user,
    GET_USER,
    "/users/{id}",
    "Get user logic"
);
http_method_macro_doc!(
    "HEAD",
    head,
    head_user,
    HEAD_USER,
    "/users/{id}",
    "Head user logic"
);
http_method_macro_doc!(
    "OPTIONS",
    options,
    options_user,
    OPTIONS_USER,
    "/users/{id}",
    "Options user logic"
);
http_method_macro_doc!(
    "PATCH",
    patch,
    patch_user,
    PATCH_USER,
    "/users/{id}",
    "Patch user logic"
);
http_method_macro_doc!(
    "POST",
    post,
    create_user,
    CREATE_USER,
    "/users",
    "Create user logic"
);
http_method_macro_doc!(
    "PUT",
    put,
    put_user,
    PUT_USER,
    "/users/{id}",
    "Put user logic"
);

/// Define a [fallback handler](https://pavex.dev/docs/guide/routing/).
///
/// A fallback handler is invoked when no other route matches the incoming request.
/// It's typically used to return a 404 Not Found response or redirect to a default page.
///
/// # Example
///
/// ```rust
/// use pavex::{fallback, Response};
///
/// #[fallback]
/// pub async fn not_found() -> Response {
///     Response::not_found()
///         .set_typed_body("The page you are looking for does not exist")
/// }
/// ```
///
/// # Guide
///
/// Check out the ["Routing"](https://pavex.dev/docs/guide/routing/)
/// section of Pavex's guide for a thorough introduction to routing
/// in Pavex applications.
///
/// # Registration
///
/// Use [`Blueprint::fallback`] to register the fallback handler with your [`Blueprint`].
///
/// The `#[fallback]` macro [generates a constant](#id) that you can use to refer to the
/// fallback handler when invoking [`Blueprint::fallback`].
///
/// # Arguments
///
/// The sections below provide an exhaustive list of all the arguments and flags supported by the [`fallback`][macro@fallback] macro:
///
/// | Name                | Kind     | Required |
/// |---------------------|----------|----------|
/// | [`id`](#id)         | Argument | No       |
///
/// ## `id`
///
/// By default, Pavex generates a constant named after your function
/// (converted to UPPER_SNAKE_CASE) that you use when registering the fallback handler.
///
/// The `id` argument allows you to customize the name of the generated constant.
///
/// ### Example
///
/// Using the default generated identifier:
///
/// ```rust
/// use pavex::{fallback, Blueprint, Response};
///
/// #[fallback]
/// pub async fn not_found() -> Response {
///     Response::not_found()
/// }
///
/// # fn main() {
/// let mut bp = Blueprint::new();
/// // The generated constant is named `NOT_FOUND`
/// bp.fallback(NOT_FOUND);
/// # }
/// ```
///
/// Using a custom identifier:
///
/// ```rust
/// use pavex::{fallback, Blueprint, Response};
///
/// #[fallback(id = "CUSTOM_404")]
/// //         👆 Custom identifier
/// pub async fn not_found() -> Response {
///     Response::not_found()
/// }
///
/// # fn main() {
/// let mut bp = Blueprint::new();
/// // Use the custom identifier when registering
/// bp.fallback(CUSTOM_404);
/// # }
/// ```
///
/// [`Blueprint::fallback`]: crate::Blueprint::fallback
/// [`Blueprint`]: crate::Blueprint
pub use pavex_macros::fallback;

/// Define a [route](https://pavex.dev/docs/guide/routing/) for requests to a given path.
///
/// The `#[route]` macro can be used to define routes that match multiple HTTP methods, or
/// non-standard ones (e.g. `QUERY`).
/// Prefer one of the short-hand attributes if you need to match a standard HTTP method:
/// [`#[get]`](crate::get), [`#[post]`](crate::post), [`#[put]`](crate::put), [`#[patch]`](crate::patch),
/// [`#[delete]`](crate::delete), [`#[head]`](crate::head), and [`#[options]`](crate::options).
///
/// # Example: Multiple methods
///
/// ```rust
/// use pavex::{route, Response};
///
/// #[route(method = ["GET", "HEAD"], path = "/users/{id}")]
/// //      👆 Multiple methods
/// pub async fn get_or_head_user(/* */) -> Response {
///     // [...]
///     # Response::ok()
/// }
/// ```
///
/// # Guide
///
/// Check out the ["Routing"](https://pavex.dev/docs/guide/routing/)
/// section of Pavex's guide for a thorough introduction to routing
/// in Pavex applications.
///
/// # Registration
///
/// You can register a route with your application in two ways:
/// - Use [`Blueprint::route`] to register a single route
/// - Use [`Blueprint::routes`] to register multiple routes in bulk
///
/// The `#[route]` macro [generates a constant](#id) that you can use to refer to the
/// route when invoking [`Blueprint::route`].
///
/// # Arguments
///
/// The sections below provide an exhaustive list of all the arguments supported by the [`route`][macro@route] macro:
///
/// | Name                      | Kind     | Required |
/// |---------------------------|----------|----------|
/// | [`method`](#method)       | Argument | Yes*     |
/// | [`path`](#path)           | Argument | Yes      |
/// | [`id`](#id)               | Argument | No       |
/// | [`allow`](#allow)         | Argument | No       |
///
/// \* The `method` argument is required unless [`allow(any_method)`](#allow) is specified.
///
/// ## `method`
///
/// The `method` argument specifies the HTTP method(s) that this route will match.
/// You can specify a single method or multiple methods.
///
/// `method` is required unless you specified [`allow(any_method)`](#allow) to accept
/// any HTTP method.
///
/// ### Example
///
/// Single method:
///
/// ```rust
/// use pavex::{route, Response};
///
/// #[route(method = "POST", path = "/users/{id}")]
/// //       👆 Single method
/// pub async fn create_user(/* */) -> Response {
///     // [...]
///     # Response::ok()
/// }
/// ```
///
/// Multiple methods:
///
/// ```rust
/// use pavex::{route, Response};
///
/// #[route(method = ["GET", "HEAD"], path = "/users/{id}")]
/// //       👆 Multiple methods
/// pub async fn get_or_head_user(/* */) -> Response {
///     // [...]
///     # Response::ok()
/// }
/// ```
///
/// ## `path`
///
/// The `path` argument specifies the URL path pattern that this route will match.
///
/// The path can contain parameter placeholders in the format `{parameter_name}` that will
/// be extracted and passed to your handler function.
///
/// ### Example
///
/// ```rust
/// use pavex::{route, Response};
///
/// #[route(method = "GET", path = "/users/{id}/posts/{post_id}")]
/// //                            👆 Path with multiple parameters
/// pub async fn get_user_post(/* */) -> Response {
///     // [...]
///     # Response::ok()
/// }
/// ```
///
/// ## `id`
///
/// By default, Pavex generates a constant named after your function
/// (converted to UPPER_SNAKE_CASE) that you use when registering the route.
///
/// The `id` argument allows you to customize the name of the generated constant.
///
/// ### Example
///
/// Using the default generated identifier:
///
/// ```rust
/// use pavex::{route, Response, Blueprint};
///
/// #[route(method = "GET", path = "/users/{id}")]
/// pub async fn get_user(/* */) -> Response {
///     // [...]
///     # Response::ok()
/// }
///
/// # fn main() {
/// let mut bp = Blueprint::new();
/// // The generated constant is named `GET_USER`
/// bp.route(GET_USER);
/// # }
/// ```
///
/// Using a custom identifier:
///
/// ```rust
/// use pavex::{route, Response, Blueprint};
///
/// #[route(method = "GET", path = "/users/{id}", id = "USER_ROUTE")]
/// //                                            👆 Custom identifier
/// pub async fn get_user(/* */) -> Response {
///     // [...]
///     # Response::ok()
/// }
///
/// # fn main() {
/// let mut bp = Blueprint::new();
/// // Use the custom identifier when registering
/// bp.route(USER_ROUTE);
/// # }
/// ```
///
/// ## `allow`
///
/// The `allow` argument can be used to enable additional behaviors for this route.
///
/// Currently, the following values are supported:
/// - `non_standard_methods`: Allow non-standard HTTP methods
/// - `any_method`: Match any HTTP method.
///   It matches non-standard methods if `non_standard_methods` is also enabled.
///
/// ### Example: Non-standard method
///
/// Allow non-standard methods:
///
/// ```rust
/// use pavex::{route, Response};
///
/// #[route(method = "QUERY", path = "/users", allow(non_standard_methods))]
/// //                                         👆 Allow non-standard methods
/// pub async fn query_users() -> Response {
///     // [...]
///     # Response::ok()
/// }
/// ```
///
/// ### Example: Any standard method
///
/// Allow any method (no need to specify `method`):
///
/// ```rust
/// use pavex::{route, Response};
///
/// #[route(path = "/webhook", allow(any_method))]
/// //                         👆 Allow any HTTP method
/// pub async fn webhook() -> Response {
///     // [...]
///     # Response::ok()
/// }
/// ```
///
/// ### Example: Arbitrary methods
///
/// Allow any method, including non-standard ones:
///
/// ```rust
/// use pavex::{route, Response};
///
/// #[route(path = "/webhook", allow(any_method, non_standard_methods))]
/// //                         👆 Allow any HTTP method, including non-standard ones
/// pub async fn webhook() -> Response {
///     // [...]
///     # Response::ok()
/// }
/// ```
///
/// [`Blueprint::route`]: crate::Blueprint::route
/// [`Blueprint::routes`]: crate::Blueprint::routes
pub use pavex_macros::route;

/// A helper macro to support Pavex attributes on methods.
///
/// You must annotate the `impl` block with `#[methods]` whenever you want to add Pavex attributes
/// like `#[get]`, `#[request_scoped]`, or `#[error_handler]` to methods defined in that `impl` block.
///
/// Without `#[methods]`, Pavex will reject the annotated methods.
///
/// # Example
///
/// ```rust
/// use pavex::methods;
/// use pavex::Response;
///
/// pub struct UserService {
///     // [...]
/// };
///
/// #[methods]  // Required for the macros below to work
/// impl UserService {
///     #[request_scoped]
///     pub fn new() -> Self {
///         // [...]
///         # Self {}
///     }
///
///     #[get(path = "/users/{id}")]
///     pub fn get_user(/* */) -> Result<Response, GetUserError> {
///         // [...]
///         # Ok(Response::ok())
///     }
///
///     #[post(path = "/users/{id}")]
///     pub fn create_user(/* */) -> Result<Response, CreateUserError> {
///         // [...]
///         # Ok(Response::ok())
///     }
/// }
/// # struct CreateUserError;
/// # struct GetUserError;
/// ```
///
/// # Supported Annotations
///
/// All Pavex attributes can be attached to both methods and functions.
pub use pavex_macros::methods;
