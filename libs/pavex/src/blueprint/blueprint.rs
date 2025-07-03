use super::Constructor;
use super::ErrorHandler;
use super::ErrorObserver;
use super::Fallback;
use super::Import;
use super::PostProcessingMiddleware;
use super::PreProcessingMiddleware;
use super::Prebuilt;
use super::RegisteredConstructor;
use super::RegisteredErrorHandler;
use super::RegisteredImport;
use super::Route;
use super::WrappingMiddleware;
use super::conversions::{coordinates2coordinates, created_at2created_at, sources2sources};
use super::nesting::NestingConditions;
use super::{
    Config, RegisteredConfig, RegisteredFallback, RegisteredPostProcessingMiddleware,
    RegisteredPreProcessingMiddleware, RegisteredRoute, RegisteredRoutes,
    RegisteredWrappingMiddleware,
};
use crate::blueprint::RegisteredErrorObserver;
use crate::blueprint::RegisteredPrebuilt;
use pavex_bp_schema::Blueprint as BlueprintSchema;
use pavex_reflection::Location;

/// The structure of your Pavex application.
///
/// # Guide
///
/// Check out the ["Project structure"](https://pavex.dev/docs/guide/project_structure) section of
/// Pavex's guide for more details on the role of [`Blueprint`] in Pavex applications.
///
/// # Overview
///
/// A blueprint keeps track of:
///
/// - [Routes](https://pavex.dev/docs/guide/routing/), registered via [`.routes()`][`Blueprint::route`] and [`.route()`][`Blueprint::route`]
/// - [Middlewares](https://pavex.dev/docs/guide/middleware/), registered via [`.pre_process()`][`Blueprint::pre_process`], [`.wrap()`][`Blueprint::wrap`] and
///   [`.post_process()`][`Blueprint::post_process`]
/// - [Error observers](https://pavex.dev/docs/guide/errors/error_observers/), registered via [`.error_observer()`][`Blueprint::error_observer`]
/// - [Constructors](https://pavex.dev/docs/guide/dependency_injection/), imported via [`.import()`][`Blueprint::import`] or registered via [`.constructor()`][`Blueprint::constructor`]
/// - [Configuration types](https://pavex.dev/docs/guide/configuration/), imported via [`.import()`][`Blueprint::import`] or registered via [`.config()`][`Blueprint::config`]
/// - [Prebuilt types](https://pavex.dev/docs/guide/dependency_injection/prebuilt_types/), imported via [`.import()`][`Blueprint::import`] or registered via [`.prebuilt()`][`Blueprint::prebuilt`]
/// - [Error handlers](https://pavex.dev/docs/guide/errors/error_handlers/), imported via [`.import()`][`Blueprint::import`] or registered via [`.error_handler()`][`Blueprint::error_handler`]
/// - Fallback routes, registered via [`.fallback()`][`Blueprint::fallback`]
///
/// You can also decompose your application into smaller sub-components
/// using [`.nest()`][`Blueprint::nest`], [`.prefix()`][`Blueprint::prefix`] and [`.domain()`][`Blueprint::domain`].
///
/// A blueprint can be serialized via [`.persist()`][`Blueprint::persist`] and forwarded to Pavex's CLI
/// to (re)generate the [server SDK crate](https://pavex.dev/docs/guide/project_structure/server_sdk/).
///
/// # Example
///
/// ```rust
/// use pavex::{Blueprint, blueprint::from};
///
/// # pub fn _blueprint(
/// # LOGGER: pavex::blueprint::WrappingMiddleware,
/// # ERROR_LOGGER: pavex::blueprint::ErrorObserver,
/// # RESPONSE_LOGGER: pavex::blueprint::PostProcessingMiddleware) {
/// let mut bp = Blueprint::new();
/// // Bring into scope constructors, error handlers and configuration
/// // types defined in the crates listed via `from!`.
/// bp.import(from![
///     // Local components, defined in this crate
///     crate,
///     // Components defined in the `pavex` crate,
///     // by the framework itself.
///     pavex,
/// ]);
///
/// // Attach a `tracing` span to every incoming request.
/// bp.wrap(LOGGER);
/// // Log the status code of every response.
/// bp.post_process(RESPONSE_LOGGER);
/// // Capture the error message and source chain
/// // of every unhandled error.
/// bp.error_observer(ERROR_LOGGER);
///
/// // Register all routes defined in this crate,
/// // prepending `/api` to their paths.
/// bp.prefix("/api").routes(from![crate]);
/// # }
/// ```
pub struct Blueprint {
    pub(super) schema: BlueprintSchema,
}

impl Default for Blueprint {
    #[track_caller]
    fn default() -> Self {
        Self {
            schema: BlueprintSchema {
                creation_location: Location::caller(),
                components: Vec::new(),
            },
        }
    }
}

impl Blueprint {
    #[track_caller]
    /// Create a new [`Blueprint`].
    pub fn new() -> Self {
        Self::default()
    }

    #[track_caller]
    /// Import all constructors, error handlers, configuration and prebuilt types defined in the target modules.
    ///
    /// Components that have been annotated with Pavex's macros (e.g. `#[singleton]`) aren't automatically
    /// considered when resolving the dependency graph for your application.\
    /// They need to be explicitly imported using one or more invocations of this method.
    ///
    /// # Guide
    ///
    /// Check out the ["Dependency Injection"](https://pavex.dev/docs/guide/dependency_injection) section of Pavex's guide
    /// for a thorough introduction to dependency injection in Pavex applications.
    ///
    /// # Wildcard import
    ///
    /// You can import all components defined in the current crate and its direct dependencies using the wildcard source, `*`:
    ///
    /// ```rust
    /// use pavex::{blueprint::from, Blueprint};
    ///
    /// # fn main() {
    /// let mut bp = Blueprint::new();
    /// bp.import(from![*]);
    /// # }
    /// ```
    ///
    /// # All local components
    ///
    /// Use `crate` as source to import all components defined in the current crate:
    ///
    /// ```rust
    /// use pavex::{blueprint::from, Blueprint};
    ///
    /// # fn main() {
    /// let mut bp = Blueprint::new();
    /// bp.import(from![crate]);
    /// # }
    /// ```
    ///
    /// # Specific modules
    ///
    /// You can restrict the import to modules:
    ///
    /// ```rust
    /// use pavex::{blueprint::from, Blueprint};
    ///
    /// # fn main() {
    /// let mut bp = Blueprint::new();
    /// // It will only import components defined
    /// // in the `crate::a` and `crate::b` modules.
    /// bp.import(from![crate::a, crate::b]);
    /// # }
    /// ```
    ///
    /// # Dependencies
    ///
    /// You can import components from a dependency using the same mechanism:
    ///
    /// ```rust
    /// use pavex::{blueprint::from, Blueprint};
    ///
    /// # fn main() {
    /// let mut bp = Blueprint::new();
    /// // Import components from the `pavex_session` and
    /// // `pavex_session_sqlx` crates.
    /// bp.import(from![pavex_session, pavex_session_sqlx]);
    /// # }
    /// ```
    ///
    /// The specified crates must be direct dependencies of the current crate.
    pub fn import(&mut self, import: Import) -> RegisteredImport {
        let import = pavex_bp_schema::Import {
            sources: sources2sources(import.sources),
            relative_to: import.relative_to.to_owned(),
            created_at: created_at2created_at(import.created_at),
            registered_at: Location::caller(),
        };
        let component_id = self.push_component(import);
        RegisteredImport {
            blueprint: &mut self.schema,
            component_id,
        }
    }

    #[track_caller]
    /// Register all the routes defined in the target modules.
    ///
    /// Components that have been annotated with Pavex's macros (e.g. `#[pavex::get]`) aren't automatically
    /// added to your application.\
    /// They need to be explicitly imported using this method or [`.route()`](Blueprint::route).
    ///
    /// # Guide
    ///
    /// Check out the ["Routing"](https://pavex.dev/docs/guide/routing) section of Pavex's guide
    /// for a thorough introduction to routing in Pavex applications.
    ///
    /// Check out [`.route()`](Blueprint::route)'s documentation to learn how routes are defined.
    ///
    /// # All local routes
    ///
    /// Use `crate` as source to register all the routes defined in the current crate:
    ///
    /// ```rust
    /// use pavex::{blueprint::from, Blueprint};
    ///
    /// # fn main() {
    /// let mut bp = Blueprint::new();
    /// bp.routes(from![crate]);
    /// # }
    /// ```
    ///
    /// # Specific modules
    ///
    /// You can restrict the scope to specific modules:
    ///
    /// ```rust
    /// use pavex::{blueprint::from, Blueprint};
    ///
    /// # fn main() {
    /// let mut bp = Blueprint::new();
    /// // It will only register routes defined
    /// // in the `crate::routes::user` and `crate::routes::post` modules.
    /// bp.routes(from![
    ///     crate::routes::user,
    ///     crate::routes::post
    /// ]);
    /// # }
    /// ```
    ///
    /// # Dependencies
    ///
    /// You can register routes defined in one of your dependencies using the same mechanism:
    ///
    /// ```rust
    /// use pavex::{blueprint::from, Blueprint};
    ///
    /// # fn main() {
    /// let mut bp = Blueprint::new();
    /// // Register request handlers from the `pavex_session` crate
    /// bp.routes(from![pavex_session]);
    /// # }
    /// ```
    ///
    /// The specified crates must be direct dependencies of the current crate.
    ///
    /// # Wildcard import
    ///
    /// You can import all routes defined in the current crate and its direct dependencies using the wildcard source, `*`:
    ///
    /// ```rust
    /// use pavex::{blueprint::from, Blueprint};
    ///
    /// # fn main() {
    /// let mut bp = Blueprint::new();
    /// bp.routes(from![*]);
    /// # }
    /// ```
    ///
    /// This is generally discouraged.
    pub fn routes(&mut self, import: Import) -> RegisteredRoutes {
        let import = pavex_bp_schema::RoutesImport {
            sources: sources2sources(import.sources),
            relative_to: import.relative_to.to_owned(),
            created_at: created_at2created_at(import.created_at),
            registered_at: Location::caller(),
        };
        let component_id = self.push_component(import);
        RegisteredRoutes {
            blueprint: &mut self.schema,
            component_id,
        }
    }

    #[track_caller]
    /// Register a route to handle incoming requests.
    ///
    /// You can register at most one route for any given [path](https://developer.mozilla.org/en-US/docs/Web/URI/Reference/Path) and
    /// [method](https://developer.mozilla.org/en-US/docs/Web/HTTP/Methods) pair.
    ///
    /// # Guide
    ///
    /// Check out the ["Routing"](https://pavex.dev/docs/guide/routing) section of Pavex's guide
    /// for a thorough introduction to routing in Pavex applications.
    ///
    /// # Example: function route
    ///
    /// Add the [`get`](macro@crate::get) attribute to a function to create a route matching `GET` requests
    /// to the given path:
    ///
    /// ```rust
    /// use pavex::get;
    /// use pavex::{request::RequestHead, response::Response};
    ///
    /// #[get(path = "/")]
    /// pub fn get_root(request_head: &RequestHead) -> Response {
    ///     // [...]
    ///     # todo!()
    /// }
    /// ```
    ///
    /// The [`get`](macro@crate::get) attribute will define a new constant,
    /// named `GET_ROOT`.\
    /// Pass the constant to [`Blueprint::route`] to add the newly-defined route to your application:
    ///
    /// ```rust
    /// # use pavex::Blueprint;
    /// # fn blueprint(GET_ROOT: pavex::blueprint::Route) {
    /// let mut bp = Blueprint::new();
    /// bp.route(GET_ROOT);
    /// # }
    /// ```
    ///
    /// ## Method-specific attributes
    ///
    /// Pavex provides attributes for the most common HTTP methods: [`get`](macro@crate::get), [`post`](macro@crate::post), [`put`](macro@crate::put),
    /// [`patch`](macro@crate::patch), [`delete`](macro@crate::delete), [`head`](macro@crate::head), and [`options`](macro@crate::options).
    /// Use the [`route`](macro@crate::route) attribute, instead, to define routes that match multiple methods,
    /// non-standard methods or arbitrary methods.
    ///
    /// # Example: method route
    ///
    /// You're not limited to free functions. Methods can be used as routes too:
    ///
    /// ```rust
    /// use pavex::methods;
    /// use pavex::request::RequestHead;
    ///
    /// pub struct LoginController(/* .. */);
    ///
    /// #[methods]
    /// impl LoginController {
    ///     #[get(path = "/login")]
    ///     pub fn get(head: &RequestHead) -> Self {
    ///         // [...]
    ///         # todo!()
    ///     }
    ///
    ///     #[post(path = "/login")]
    ///     pub fn post(head: &RequestHead) -> Self {
    ///         // [...]
    ///         # todo!()
    ///     }
    /// }
    /// ```
    ///
    /// For methods, you must add a `#[methods]` annotation on the `impl` block it belongs to,
    /// in addition to the verb annotation on the method itself.\
    /// The generated constant is named `<type_name>_<method_name>`, in constant case:
    ///
    /// ```rust
    /// # use pavex::Blueprint;
    /// # fn blueprint(LOGIN_CONTROLLER_GET: pavex::blueprint::Route, LOGIN_CONTROLLER_POST: pavex::blueprint::Route) {
    /// let mut bp = Blueprint::new();
    /// bp.route(LOGIN_CONTROLLER_GET);
    /// bp.route(LOGIN_CONTROLLER_POST);
    /// # }
    /// ```
    ///
    /// # Imports
    ///
    /// If you have defined multiple routes, you can invoke [`.routes()`][`Blueprint::routes`]
    /// to register them in bulk:
    ///
    /// ```rust
    /// use pavex::{Blueprint, blueprint::from};
    ///
    /// let mut bp = Blueprint::new();
    /// // Import all the routes defined in the current crate.
    /// // It's equivalent to invoking `bp.route` for every
    /// // single route defined in the current crate.
    /// bp.routes(from![crate]);
    /// ```
    ///
    /// Check out the documentation for [`.routes()`][`Blueprint::routes`] for more information.
    pub fn route(&mut self, route: Route) -> RegisteredRoute {
        let registered = pavex_bp_schema::Route {
            coordinates: coordinates2coordinates(route.coordinates),
            registered_at: Location::caller(),
            error_handler: None,
        };
        let component_id = self.push_component(registered);
        RegisteredRoute {
            blueprint: &mut self.schema,
            component_id,
        }
    }

    #[track_caller]
    /// Add a new type to the application's configuration.
    ///
    /// # Required traits
    ///
    /// Configuration types *must* implement `Debug`, `Clone` and `serde::Deserialize`.
    ///
    /// # Guide
    ///
    /// Check out the ["Configuration"](https://pavex.dev/docs/guide/configuration)
    /// section of Pavex's guide for a thorough introduction to Pavex's configuration system.
    ///
    /// # Example
    ///
    /// Add the [`config`](macro@crate::config) attribute to the type you want to include in
    /// the configuration for your application:
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
    /// The [`config`](macro@crate::config) attribute will define a new constant, named `POOL_CONFIG`.\
    /// Pass the constant to [`Blueprint::config`] to add the new configuration type to your application:
    ///
    /// ```rust
    /// # use pavex::Blueprint;
    /// # fn blueprint(POOL_CONFIG: pavex::blueprint::Config) {
    /// let mut bp = Blueprint::new();
    /// bp.config(POOL_CONFIG);
    /// # }
    /// ```
    ///
    /// A new field, named `pool` with type `PoolConfig`, will be added to the generated `ApplicationConfig` struct.
    ///
    /// # Imports
    ///
    /// If you have defined multiple configuration types, you can use an [import](`Blueprint::import`)
    /// to register them in bulk:
    ///
    /// ```rust
    /// use pavex::{Blueprint, blueprint::from};
    ///
    /// let mut bp = Blueprint::new();
    /// // Import all the types from the current crate that
    /// // have been annotated with `#[config]`.
    /// // It's equivalent to calling `bp.config` for
    /// // every single configuration type defined in the current crate.
    /// bp.import(from![crate]);
    /// ```
    ///
    /// Check out the documentation for [`Blueprint::import`] for more information.
    pub fn config(&mut self, config: Config) -> RegisteredConfig {
        let registered = pavex_bp_schema::ConfigType {
            coordinates: coordinates2coordinates(config.coordinates),
            cloning_strategy: None,
            default_if_missing: None,
            include_if_unused: None,
            registered_at: Location::caller(),
        };
        let component_id = self.push_component(registered);
        RegisteredConfig {
            blueprint: &mut self.schema,
            component_id,
        }
    }

    #[track_caller]
    /// Register a constructor.
    ///
    /// If a constructor for the same type has already been registered, it will be overwritten.
    ///
    /// # Guide
    ///
    /// Check out the ["Dependency injection"](https://pavex.dev/docs/guide/dependency_injection)
    /// section of Pavex's guide for a thorough introduction to dependency injection
    /// in Pavex applications.
    ///
    /// # Example: function constructor
    ///
    /// Add the [`request_scoped`](macro@crate::request_scoped) attribute to a function to mark it as a
    /// [request-scoped](crate::blueprint::Lifecycle) constructor:
    ///
    /// ```rust
    /// use pavex::request_scoped;
    /// use pavex::request::RequestHead;
    ///
    /// # struct LogLevel;
    /// pub struct AuthorizationHeader(/* .. */);
    ///
    /// #[request_scoped]
    /// pub fn extract_authorization(head: &RequestHead) -> AuthorizationHeader {
    ///     // [...]
    ///     # todo!()
    /// }
    /// ```
    ///
    /// The [`request_scoped`](macro@crate::request_scoped) attribute will define a new constant,
    /// named `EXTRACT_AUTHORIZATION`.\
    /// Pass the constant to [`Blueprint::constructor`] to allow other components to inject an instance
    /// of the `AuthorizationHeader` type as an input parameter.
    ///
    /// ```rust
    /// # use pavex::Blueprint;
    /// # fn blueprint(EXTRACT_AUTHORIZATION: pavex::blueprint::Constructor) {
    /// let mut bp = Blueprint::new();
    /// bp.constructor(EXTRACT_AUTHORIZATION);
    /// # }
    /// ```
    ///
    /// ## Lifecycles
    ///
    /// You can also register constructors with [singleton](crate::blueprint::Lifecycle::Singleton) and
    /// [transient](crate::blueprint::Lifecycle::Transient) lifecycles. Check out the respective
    /// macros ([`singleton`](macro@crate::singleton) and [`transient`](macro@crate::transient)) for more
    /// details.
    ///
    /// # Example: method constructor
    ///
    /// You're not limited to free functions. Methods can be used as constructors too:
    ///
    /// ```rust
    /// use pavex::methods;
    /// use pavex::request::RequestHead;
    ///
    /// # struct LogLevel;
    /// pub struct AuthorizationHeader(/* .. */);
    ///
    /// #[methods]
    /// impl AuthorizationHeader {
    ///     #[request_scoped]
    ///     pub fn new(head: &RequestHead) -> Self {
    ///         // [...]
    ///         # todo!()
    ///     }
    /// }
    /// ```
    ///
    /// For methods, you must add a `#[methods]` annotation on the `impl` block it belongs to,
    /// in addition to the `#[request_scoped]` annotation on the method itself.\
    ///
    /// The generated constant is named `<type_name>_<method_name>`, in constant case:
    ///
    /// ```rust
    /// # use pavex::Blueprint;
    /// # fn blueprint(AUTHORIZATION_HEADER_NEW: pavex::blueprint::Constructor) {
    /// let mut bp = Blueprint::new();
    /// bp.constructor(AUTHORIZATION_HEADER_NEW);
    /// # }
    /// ```
    ///
    /// # Imports
    ///
    /// If you have defined multiple constructors, you can use an [import](`Blueprint::import`)
    /// to register them in bulk:
    ///
    /// ```rust
    /// use pavex::{Blueprint, blueprint::from};
    ///
    /// let mut bp = Blueprint::new();
    /// // Import all the types from the current crate that
    /// // have been annotated with either `#[singleton]`,
    /// // `#[request_scoped]`, `#[transient]` or `#[constructor]`.
    /// // It's equivalent to invoking `bp.constructor` for every
    /// // single constructor defined in the current crate.
    /// bp.import(from![crate]);
    /// ```
    ///
    /// Check out the documentation for [`Blueprint::import`] for more information.
    pub fn constructor(&mut self, constructor: Constructor) -> RegisteredConstructor {
        let registered_constructor = pavex_bp_schema::Constructor {
            coordinates: coordinates2coordinates(constructor.coordinates),
            lifecycle: None,
            cloning_strategy: None,
            error_handler: None,
            lints: Default::default(),
            registered_at: Location::caller(),
        };
        let component_id = self.push_component(registered_constructor);
        RegisteredConstructor {
            component_id,
            blueprint: &mut self.schema,
        }
    }

    #[track_caller]
    /// Register a wrapping middleware.
    ///
    /// # Guide
    ///
    /// Check out the ["Middleware"](https://pavex.dev/docs/guide/middleware)
    /// section of Pavex's guide for a thorough introduction to middlewares
    /// in Pavex applications.
    ///
    /// # Example: function wrapper
    ///
    /// Add the [`wrap`](macro@crate::wrap) attribute to a function to mark it as a
    /// a wrapping middleware:
    ///
    /// ```rust
    /// use pavex::{middleware::Next, response::Response, wrap};
    /// use std::time::Duration;
    /// use tokio::time::{timeout, error::Elapsed};
    ///
    /// #[wrap]
    /// pub async fn timeout_wrapper<C>(next: Next<C>) -> Result<Response, Elapsed>
    /// where
    ///     C: IntoFuture<Output = Response>
    /// {
    ///     timeout(Duration::from_secs(2), next.into_future()).await
    /// }
    /// ```
    ///
    /// The [`wrap`](macro@crate::wrap) attribute will define a new constant,
    /// named `TIMEOUT_WRAPPER`.\
    /// Pass the constant to [`Blueprint::wrap`] to add the newly-defined middleware to
    /// your application:
    ///
    /// ```rust
    /// # use pavex::Blueprint;
    /// # fn blueprint(TIMEOUT_WRAPPER: pavex::blueprint::WrappingMiddleware) {
    /// let mut bp = Blueprint::new();
    /// bp.wrap(TIMEOUT_WRAPPER);
    /// # }
    /// ```
    ///
    /// # Example: method middleware
    ///
    /// You're not limited to free functions. Methods can be used as middlewares too:
    ///
    /// ```rust
    /// use pavex::{middleware::Next, response::Response, methods};
    /// use std::time::Duration;
    /// use tokio::time::{timeout, error::Elapsed};
    ///
    /// pub struct TimeoutMiddleware {
    ///     timeout: Duration,
    /// }
    ///
    /// #[methods]
    /// impl TimeoutMiddleware {
    ///     #[wrap]
    ///     pub async fn execute<C>(&self, next: Next<C>) -> Result<Response, Elapsed>
    ///     where
    ///         C: IntoFuture<Output = Response>
    ///     {
    ///         timeout(self.timeout, next.into_future()).await
    ///     }
    /// }
    /// ```
    ///
    /// For methods, you must add a `#[methods]` annotation on the `impl` block it belongs to,
    /// in addition to the `#[wrap]` annotation on the method itself.\
    /// The generated constant is named `<type_name>_<method_name>`, in constant case:
    ///
    /// ```rust
    /// # use pavex::Blueprint;
    /// # fn blueprint(TIMEOUT_MIDDLEWARE_EXECUTE: pavex::blueprint::WrappingMiddleware) {
    /// let mut bp = Blueprint::new();
    /// bp.wrap(TIMEOUT_MIDDLEWARE_EXECUTE);
    /// # }
    /// ```
    #[doc(alias = "middleware")]
    pub fn wrap(&mut self, m: WrappingMiddleware) -> RegisteredWrappingMiddleware {
        let registered = pavex_bp_schema::WrappingMiddleware {
            coordinates: coordinates2coordinates(m.coordinates),
            registered_at: Location::caller(),
            error_handler: None,
        };
        let component_id = self.push_component(registered);
        RegisteredWrappingMiddleware {
            blueprint: &mut self.schema,
            component_id,
        }
    }

    #[track_caller]
    /// Register a post-processing middleware.
    ///
    /// # Guide
    ///
    /// Check out the ["Middleware"](https://pavex.dev/docs/guide/middleware)
    /// section of Pavex's guide for a thorough introduction to middlewares
    /// in Pavex applications.
    ///
    /// # Example: function middleware
    ///
    /// Add the [`post_process`](macro@crate::post_process) attribute to a function to mark it as a
    /// a post-processing middleware:
    ///
    /// ```rust
    /// use pavex::{post_process, response::Response};
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
    /// The [`post_process`](macro@crate::post_process) attribute will define a new constant,
    /// named `RESPONSE_LOGGER`.\
    /// Pass the constant to [`Blueprint::post_process`] to add the newly-defined middleware to
    /// your application:
    ///
    /// ```rust
    /// # use pavex::Blueprint;
    /// # fn blueprint(RESPONSE_LOGGER: pavex::blueprint::PostProcessingMiddleware) {
    /// let mut bp = Blueprint::new();
    /// bp.post_process(RESPONSE_LOGGER);
    /// # }
    /// ```
    ///
    /// # Example: method middleware
    ///
    /// You're not limited to free functions. Methods can be used as middlewares too:
    ///
    /// ```rust
    /// use pavex::{methods, response::Response};
    /// use pavex_tracing::{
    ///     RootSpan,
    ///     fields::{http_response_status_code, HTTP_RESPONSE_STATUS_CODE}
    /// };
    ///
    /// pub struct ResponseLogger {
    ///     log_body_size: bool,
    /// }
    ///
    /// #[methods]
    /// impl ResponseLogger {
    ///     #[post_process]
    ///     pub fn log(&self, response: Response, root_span: &RootSpan) -> Response
    ///     {
    ///         if self.log_body_size {
    ///             // [...]
    ///         }
    ///         root_span.record(
    ///             HTTP_RESPONSE_STATUS_CODE,
    ///             http_response_status_code(&response),
    ///         );
    ///         response
    ///     }
    /// }
    /// ```
    ///
    /// For methods, you must add a [`#[methods]`][macro@crate::methods] annotation on the `impl` block it belongs to,
    /// in addition to the [`#[post_process]`][macro@crate::post_process] annotation on the method itself.\
    /// The generated constant is named `<type_name>_<method_name>`, in constant case:
    ///
    /// ```rust
    /// # use pavex::Blueprint;
    /// # fn blueprint(RESPONSE_LOGGER_LOG: pavex::blueprint::PostProcessingMiddleware) {
    /// let mut bp = Blueprint::new();
    /// bp.post_process(RESPONSE_LOGGER_LOG);
    /// # }
    /// ```
    #[doc(alias = "middleware")]
    #[doc(alias = "postprocess")]
    pub fn post_process(
        &mut self,
        m: PostProcessingMiddleware,
    ) -> RegisteredPostProcessingMiddleware {
        let registered = pavex_bp_schema::PostProcessingMiddleware {
            coordinates: coordinates2coordinates(m.coordinates),
            registered_at: Location::caller(),
            error_handler: None,
        };
        let component_id = self.push_component(registered);
        RegisteredPostProcessingMiddleware {
            blueprint: &mut self.schema,
            component_id,
        }
    }

    #[track_caller]
    /// Register a pre-processing middleware.
    ///
    /// # Guide
    ///
    /// Check out the ["Middleware"](https://pavex.dev/docs/guide/middleware)
    /// section of Pavex's guide for a thorough introduction to middlewares
    /// in Pavex applications.
    ///
    /// # Example: function middleware
    ///
    /// Add the [`pre_process`](macro@crate::pre_process) attribute to a function to mark it as a
    /// a pre-processing middleware:
    ///
    /// ```rust
    /// use pavex::{Blueprint, pre_process, response::Response};
    /// use pavex::middleware::Processing;
    /// use pavex::http::{HeaderValue, header::LOCATION};
    /// use pavex::request::RequestHead;
    ///
    /// /// If the request path ends with a `/`,
    /// /// redirect to the same path without the trailing `/`.
    /// #[pre_process]
    /// pub fn redirect_to_normalized(request_head: &RequestHead) -> Processing
    /// {
    ///     let Some(normalized_path) = request_head.target.path().strip_suffix('/') else {
    ///         // No need to redirect, we continue processing the request.
    ///         return Processing::Continue;
    ///     };
    ///     let location = HeaderValue::from_str(normalized_path).unwrap();
    ///     let redirect = Response::temporary_redirect().insert_header(LOCATION, location);
    ///     // Short-circuit the request processing pipeline and return the redirect response
    ///     // to the client without invoking downstream middlewares and the request handler.
    ///     Processing::EarlyReturn(redirect)
    /// }
    /// ```
    ///
    /// The [`pre_process`](macro@crate::pre_process) attribute will define a new constant,
    /// named `REDIRECT_TO_NORMALIZED`.\
    /// Pass the constant to [`Blueprint::pre_process`] to add the newly-defined middleware to
    /// your application:
    ///
    /// ```rust
    /// # use pavex::Blueprint;
    /// # fn blueprint(REDIRECT_TO_NORMALIZED: pavex::blueprint::PreProcessingMiddleware) {
    /// let mut bp = Blueprint::new();
    /// bp.pre_process(REDIRECT_TO_NORMALIZED);
    /// # }
    /// ```
    ///
    /// # Example: method middleware
    ///
    /// You're not limited to free functions. Methods can be used as middlewares too:
    ///
    /// ```rust
    /// use pavex::{methods, response::Response};
    /// use pavex::middleware::Processing;
    /// use pavex::http::{HeaderValue, header::LOCATION};
    /// use pavex::request::RequestHead;
    ///
    /// pub struct PathNormalizer {
    ///     // [...]
    /// }
    ///
    /// #[methods]
    /// impl PathNormalizer {
    ///     #[pre_process]
    ///     pub fn redirect(request_head: &RequestHead) -> Processing
    ///     {
    ///         // [...]
    ///         # todo!()
    ///     }
    /// }
    /// ```
    ///
    /// For methods, you must add a [`#[methods]`][macro@crate::methods] annotation on the `impl` block it belongs to,
    /// in addition to the [`#[pre_process]`][macro@crate::pre_process] annotation on the method itself.\
    /// The generated constant is named `<type_name>_<method_name>`, in constant case:
    ///
    /// ```rust
    /// # use pavex::Blueprint;
    /// # fn blueprint(PATH_NORMALIZER_REDIRECT: pavex::blueprint::PreProcessingMiddleware) {
    /// let mut bp = Blueprint::new();
    /// bp.pre_process(PATH_NORMALIZER_REDIRECT);
    /// # }
    /// ```
    #[doc(alias = "middleware")]
    #[doc(alias = "preprocess")]
    pub fn pre_process(&mut self, m: PreProcessingMiddleware) -> RegisteredPreProcessingMiddleware {
        let registered = pavex_bp_schema::PreProcessingMiddleware {
            coordinates: coordinates2coordinates(m.coordinates),
            registered_at: Location::caller(),
            error_handler: None,
        };
        let component_id = self.push_component(registered);
        RegisteredPreProcessingMiddleware {
            blueprint: &mut self.schema,
            component_id,
        }
    }

    /// Nest a [`Blueprint`] under the current [`Blueprint`] (the parent), without adding a [common path prefix](Self::prefix)
    /// nor a [domain restriction](Self::domain) to its routes.
    ///
    /// Check out [`NestingConditions::nest`](super::nesting::NestingConditions::nest) for more details on nesting.
    #[track_caller]
    #[doc(alias("scope"))]
    pub fn nest(&mut self, blueprint: Blueprint) {
        self.push_component(pavex_bp_schema::NestedBlueprint {
            blueprint: blueprint.schema,
            path_prefix: None,
            domain: None,
            nested_at: Location::caller(),
        });
    }

    #[track_caller]
    /// A common prefix will be prepended to the path of routes nested under this condition.
    ///
    /// ```rust
    /// use pavex::Blueprint;
    /// use pavex::get;
    /// use pavex::response::Response;
    ///
    /// fn app() -> Blueprint {
    ///     let mut bp = Blueprint::new();
    ///     // Adding `/api` as common prefix here
    ///     bp.prefix("/api").nest(api_bp());
    ///     bp
    /// }
    ///
    /// #[get(path = "/version")]
    /// pub fn get_api_version() -> Response {
    ///     // [...]
    ///     # todo!()
    /// }
    ///
    /// fn api_bp() -> Blueprint {
    ///     let mut bp = Blueprint::new();
    ///     // This will match `GET` requests to `/api/version`.
    ///     bp.route(GET_API_VERSION);
    ///     bp
    /// }
    /// # pub fn handler() {}
    /// ```
    ///
    /// You can also add a (sub)domain constraint, in addition to the common prefix:
    ///
    /// ```rust
    /// use pavex::Blueprint;
    /// use pavex::get;
    /// use pavex::response::Response;
    ///
    /// fn app() -> Blueprint {
    ///    let mut bp = Blueprint::new();
    ///    bp.prefix("/v1").domain("api.mybusiness.com").nest(api_bp());
    ///    bp
    /// }
    ///
    /// #[get(path = "/about")]
    /// pub fn get_about() -> Response {
    ///     // [...]
    ///     # todo!()
    /// }
    ///
    /// fn api_bp() -> Blueprint {
    ///    let mut bp = Blueprint::new();
    ///   // This will match `GET` requests to `api.mybusiness.com/v1/about`.
    ///   bp.route(GET_ABOUT);
    ///   bp
    /// }
    /// ```
    ///
    /// Check out [`Blueprint::domain`] for more details on domain restrictions.
    ///
    /// ## Restrictions
    ///
    /// `prefix` must be non-empty and it must start with a `/`.
    /// If you don't want to add a common prefix, check out [`Blueprint::nest`] or [`Blueprint::domain`].
    ///
    /// ## Trailing slashes
    ///
    /// `prefix` **can't** end with a trailing `/`.
    /// This would result in routes with two consecutive `/` in their paths—e.g.
    /// `/prefix//path`—which is rarely desirable.
    /// If you actually need consecutive slashes in your route, you can add them explicitly to
    /// the path of the route registered in the nested blueprint:
    ///
    /// ```rust
    /// use pavex::Blueprint;
    /// use pavex::get;
    /// use pavex::response::Response;
    ///
    /// fn app() -> Blueprint {
    ///     let mut bp = Blueprint::new();
    ///     bp.prefix("/api").nest(api_bp());
    ///     bp
    /// }
    ///
    /// #[get(path = "//version")]
    /// pub fn get_api_version() -> Response {
    ///     // [...]
    ///     # todo!()
    /// }
    ///
    /// fn api_bp() -> Blueprint {
    ///     let mut bp = Blueprint::new();
    ///     // This will match `GET` requests to `/api//version`.
    ///     bp.route(GET_API_VERSION);
    ///     bp
    /// }
    /// # pub fn handler() {}
    /// ```
    pub fn prefix(&mut self, prefix: &str) -> NestingConditions {
        NestingConditions::empty(&mut self.schema).prefix(prefix)
    }

    #[track_caller]
    /// Only requests to the specified domain will be forwarded to routes nested under this condition.
    ///
    /// # Example
    ///
    /// ```rust
    /// use pavex::Blueprint;
    /// # fn api_routes() -> Blueprint { Blueprint::new() }
    /// # fn console_routes() -> Blueprint { Blueprint::new() }
    ///
    /// let mut bp = Blueprint::new();
    ///
    /// // We split UI and API routes into separate blueprints,
    /// // and we serve them using different subdomains.
    /// bp.domain("api.mybusiness.com")
    ///   .nest(api_routes());
    /// bp.domain("console.mybusiness.com")
    ///   .nest(console_routes());
    /// ```
    ///
    /// You can also prepend a common path prefix to all registered routes, in addition to the
    /// domain constraint:
    ///
    /// ```rust
    /// use pavex::Blueprint;
    /// use pavex::get;
    /// use pavex::response::Response;
    ///
    /// fn app() -> Blueprint {
    ///    let mut bp = Blueprint::new();
    ///    bp.prefix("/v1").domain("api.mybusiness.com").nest(api_bp());
    ///    bp
    /// }
    ///
    /// #[get(path = "/about")]
    /// pub fn get_about() -> Response {
    ///     // [...]
    ///     # todo!()
    /// }
    ///
    /// fn api_bp() -> Blueprint {
    ///    let mut bp = Blueprint::new();
    ///   // This will match `GET` requests to `api.mybusiness.com/v1/about`.
    ///   bp.route(GET_ABOUT);
    ///   bp
    /// }
    /// ```
    ///
    /// Check out [`Blueprint::prefix`] for more details on path prefixes.
    ///
    /// # Domain detection
    ///
    /// Domain detection is based on the value of [`Host` header](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Host).
    /// If the header is not present in the request, the condition will be considered as not met.
    ///
    /// Keep in mind that the [`Host` header can be easily spoofed by the client](https://owasp.org/www-project-web-security-testing-guide/latest/4-Web_Application_Security_Testing/07-Input_Validation_Testing/17-Testing_for_Host_Header_Injection),
    /// so you should not rely on its value for auth or other security-sensitive operations.
    pub fn domain(&mut self, domain: &str) -> NestingConditions {
        NestingConditions::empty(&mut self.schema).domain(domain)
    }

    #[track_caller]
    /// Register a fallback handler to be invoked when an incoming request does **not** match
    /// any of the routes you registered with [`Blueprint::route`].
    ///
    /// If you don't register a fallback handler, the
    /// [default framework fallback](crate::router::default_fallback) will be used instead.
    ///
    /// If a fallback handler has already been registered against this `Blueprint`,
    /// it will be overwritten.
    ///
    /// # Example
    ///
    /// ```rust
    /// use pavex::{get, fallback, Blueprint};
    /// use pavex::response::Response;
    ///
    /// #[get(path = "/path")]
    /// pub fn get_path() -> Response {
    ///     // [...]
    ///     # todo!()
    /// }
    /// #[fallback]
    /// pub fn fallback_handler() -> Response {
    ///     // [...]
    ///     # todo!()
    /// }
    ///
    /// # fn main() {
    /// let mut bp = Blueprint::new();
    /// bp.route(GET_PATH);
    /// // The fallback handler will be invoked for all the requests that don't match `/path`.
    /// // E.g. `GET /home`, `POST /home`, `GET /home/123`, etc.
    /// bp.fallback(FALLBACK_HANDLER);
    /// # }
    /// ```
    ///
    /// # Signature
    ///
    /// A fallback handler is a function (or a method) that returns a [`Response`], either directly
    /// (if infallible) or wrapped in a [`Result`] (if fallible).
    ///
    /// Fallback handlers can take advantage of dependency injection, like any
    /// other component.
    /// You list what you want to see injected as function parameters
    /// and Pavex will inject them for you in the generated code.
    ///
    /// ## Nesting
    ///
    /// You can register a single fallback handler for each blueprint.
    /// If your application takes advantage of [nesting](Blueprint::nest), you can register
    /// a fallback against each nested blueprint in your application as well as one for the
    /// top-level blueprint.
    ///
    /// Let's explore how nesting affects the invocation of fallback handlers.
    ///
    /// ### Nesting without prefix
    ///
    /// The fallback registered against a blueprint will be invoked for all the requests that match
    /// the path of a route that was **directly** registered against that blueprint, but don't satisfy
    /// their method guards.
    ///
    /// ```rust
    /// use pavex::{get, fallback, Blueprint};
    /// use pavex::response::Response;
    ///
    /// #[get(path = "/home")]
    /// pub fn get_home() -> Response {
    ///     // [...]
    ///     # todo!()
    /// }
    ///
    /// #[get(path = "/room")]
    /// pub fn get_room() -> Response {
    ///     // [...]
    ///     # todo!()
    /// }
    ///
    /// #[fallback]
    /// pub fn fallback_handler() -> Response {
    ///     // [...]
    ///     # todo!()
    /// }
    ///
    /// # fn main() {
    /// let mut bp = Blueprint::new();
    /// bp.route(GET_HOME);
    /// bp.nest({
    ///     let mut bp = Blueprint::new();
    ///     bp.route(GET_ROOM);
    ///     bp.fallback(FALLBACK_HANDLER);
    ///     bp
    /// });
    /// # }
    /// ```
    ///
    /// In the example above, `fallback_handler` will be invoked for incoming `POST /room`
    /// requests: the path matches the path of a route registered against the nested blueprint
    /// (`GET /room`), but the method guard doesn't (`POST` vs `GET`).
    /// If the incoming requests don't have `/room` as their path instead (e.g. `GET /street`
    /// or `GET /room/123`), they will be handled by the fallback registered against the **parent**
    /// blueprint—the top-level one in this case.
    /// Since no fallback has been explicitly registered against the top-level blueprint, the
    /// [default framework fallback](crate::router::default_fallback) will be used instead.
    ///
    /// ### Nesting with prefix
    ///
    /// If the nested blueprint includes a nesting prefix (e.g. `bp.nest_at("/api", api_bp)`),
    /// its fallback will **also** be invoked for all the requests that start with the prefix
    /// but don't match any of the route paths registered against the nested blueprint.
    ///
    /// ```rust
    /// use pavex::{get, fallback, Blueprint};
    /// use pavex::response::Response;
    ///
    /// #[get(path = "/home")]
    /// pub fn get_home() -> Response {
    ///     // [...]
    ///     # todo!()
    /// }
    ///
    /// #[get(path = "/")]
    /// pub fn list_rooms() -> Response {
    ///     // [...]
    ///     # todo!()
    /// }
    ///
    /// #[fallback]
    /// pub fn fallback_handler() -> Response {
    ///     // [...]
    ///     # todo!()
    /// }
    ///
    /// # fn main() {
    /// let mut bp = Blueprint::new();
    /// bp.route(GET_HOME);
    /// bp.prefix("/room").nest({
    ///     let mut bp = Blueprint::new();
    ///     bp.route(LIST_ROOMS);
    ///     bp.fallback(FALLBACK_HANDLER);
    ///     bp
    /// });
    /// # }
    /// ```
    ///
    /// In the example above, `fallback_handler` will be invoked for both `POST /room`
    /// **and** `POST /room/123` requests: the path of the latter doesn't match the path of the only
    /// route registered against the nested blueprint (`GET /room/`), but it starts with the
    /// prefix of the nested blueprint (`/room`).
    ///
    /// [`Response`]: crate::response::Response
    pub fn fallback(&mut self, fallback: Fallback) -> RegisteredFallback {
        let registered = pavex_bp_schema::Fallback {
            coordinates: coordinates2coordinates(fallback.coordinates),
            registered_at: Location::caller(),
            error_handler: None,
        };
        let component_id = self.push_component(registered);
        RegisteredFallback {
            blueprint: &mut self.schema,
            component_id,
        }
    }

    #[track_caller]
    /// Register an error observer to intercept and report errors that occur during request handling.
    ///
    /// # Guide
    ///
    /// Check out the ["Error observers"](https://pavex.dev/docs/guide/errors/error_observers)
    /// section of Pavex's guide for a thorough introduction to error observers
    /// in Pavex applications.
    ///
    /// # Example: function observer
    ///
    /// ```rust
    /// use pavex::error_observer;
    /// use tracing_log_error::log_error;
    ///
    /// #[error_observer]
    /// pub fn error_logger(e: &pavex::Error) {
    ///     log_error!(e, "An error occurred while handling a request");
    /// }
    /// ```
    ///
    /// The [`error_observer`](macro@crate::error_observer) attribute will define a new constant,
    /// named `ERROR_LOGGER`.\
    /// Pass the constant to [`.error_observer()`][`Blueprint::error_observer`] to register
    /// the newly defined error observer:
    ///
    /// ```rust
    /// # use pavex::Blueprint;
    /// # fn blueprint(ERROR_LOGGER: pavex::blueprint::ErrorObserver) {
    /// let mut bp = Blueprint::new();
    /// bp.error_observer(ERROR_LOGGER);
    /// # }
    /// ```
    ///
    /// # Example: method observer
    ///
    /// You're not limited to free functions. Methods can be used as error observers too:
    ///
    /// ```rust
    /// use pavex::methods;
    /// use tracing_log_error::log_error;
    ///
    /// pub struct ErrorLogger;
    ///
    /// #[methods]
    /// impl ErrorLogger {
    ///     #[error_observer]
    ///     pub fn log(e: &pavex::Error) {
    ///         log_error!(e, "An error occurred while handling a request");
    ///     }
    /// }
    /// ```
    ///
    /// For methods, you must add a [`#[methods]`](macro@crate::methods) annotation on the `impl` block it belongs to,
    /// in addition to the [`#[error_observer]`](macro@crate::error_observer) annotation on the method itself.\
    /// The generated constant is named `<type_name>_<method_name>`, in constant case:
    ///
    /// ```rust
    /// # use pavex::Blueprint;
    /// # fn blueprint(ERROR_LOGGER_LOG: pavex::blueprint::ErrorObserver) {
    /// let mut bp = Blueprint::new();
    /// bp.error_observer(ERROR_LOGGER_LOG);
    /// # }
    /// ```
    pub fn error_observer(&mut self, error_observer: ErrorObserver) -> RegisteredErrorObserver {
        let registered = pavex_bp_schema::ErrorObserver {
            coordinates: coordinates2coordinates(error_observer.coordinates),
            registered_at: Location::caller(),
        };
        self.push_component(registered);
        RegisteredErrorObserver {
            blueprint: &mut self.schema,
        }
    }

    #[track_caller]
    /// Register an error handler.
    ///
    /// # Guide
    ///
    /// Check out the ["Error handlers"](https://pavex.dev/docs/guide/errors/error_handlers)
    /// section of Pavex's guide for a thorough introduction to error handlers
    /// in Pavex applications.
    ///
    /// # Example: function handler
    ///
    /// Add the [`error_handler`](macro@crate::error_handler) attribute to a function to mark it as
    /// an error handler:
    ///
    /// ```rust
    /// use pavex::error_handler;
    /// use pavex::response::Response;
    ///
    /// pub enum LoginError {
    ///     InvalidCredentials,
    ///     DatabaseError,
    /// }
    ///
    /// #[error_handler]
    /// pub fn login_error_handler(e: &LoginError) -> Response {
    ///     match e {
    ///         LoginError::InvalidCredentials => Response::unauthorized(),
    ///         LoginError::DatabaseError => Response::internal_server_error(),
    ///     }
    /// }
    ///```
    ///
    /// The [`error_handler`](macro@crate::error_handler) attribute will define a new constant,
    /// named `LOGIN_ERROR_HANDLER`.\
    /// Pass the constant to [`.error_handler()`][`Blueprint::error_handler`] to register
    /// the newly defined error handler:
    ///
    /// ```rust
    /// # use pavex::Blueprint;
    /// # fn blueprint(LOGIN_ERROR_HANDLER: pavex::blueprint::ErrorHandler) {
    /// let mut bp = Blueprint::new();
    /// bp.error_handler(LOGIN_ERROR_HANDLER);
    /// # }
    /// ```
    ///
    /// # Example: method handler
    ///
    /// You're not limited to free functions. Methods can be used as error handlers too:
    ///
    /// ```rust
    /// use pavex::methods;
    /// use pavex::response::Response;
    ///
    /// pub enum LoginError {
    ///     InvalidCredentials,
    ///     DatabaseError,
    /// }
    ///
    /// #[methods]
    /// impl LoginError {
    ///     #[error_handler]
    ///     pub fn to_response(&self) -> Response {
    ///         match self {
    ///             LoginError::InvalidCredentials => Response::unauthorized(),
    ///             LoginError::DatabaseError => Response::internal_server_error(),
    ///         }
    ///     }
    /// }
    /// ```
    ///
    /// For methods, you must add a [`#[methods]`](macro@crate::methods) annotation on the `impl` block it belongs to,
    /// in addition to the [`#[error_handler]`](macro@crate::error_handler) annotation on the method itself.\
    /// The generated constant is named `<type_name>_<method_name>`, in constant case:
    ///
    /// ```rust
    /// # use pavex::Blueprint;
    /// # fn blueprint(LOGIN_ERROR_TO_RESPONSE: pavex::blueprint::ErrorHandler) {
    /// let mut bp = Blueprint::new();
    /// bp.error_handler(LOGIN_ERROR_TO_RESPONSE);
    /// # }
    /// ```
    pub fn error_handler(&mut self, m: ErrorHandler) -> RegisteredErrorHandler {
        let registered = pavex_bp_schema::ErrorHandler {
            coordinates: coordinates2coordinates(m.coordinates),
            registered_at: Location::caller(),
        };
        self.push_component(registered);
        RegisteredErrorHandler {
            blueprint: &mut self.schema,
        }
    }

    #[track_caller]
    /// Register a type to be used as input parameter to the (generated) `ApplicationState::new`
    /// method.
    ///
    /// # Guide
    ///
    /// Check out the ["Dependency injection"](https://pavex.dev/docs/guide/dependency_injection)
    /// section of Pavex's guide for a thorough introduction to dependency injection
    /// in Pavex applications.
    pub fn prebuilt(&mut self, prebuilt: Prebuilt) -> RegisteredPrebuilt {
        let registered = pavex_bp_schema::PrebuiltType {
            coordinates: coordinates2coordinates(prebuilt.coordinates),
            cloning_strategy: None,
            registered_at: Location::caller(),
        };
        let component_id = self.push_component(registered);
        RegisteredPrebuilt {
            blueprint: &mut self.schema,
            component_id,
        }
    }

    /// Register a component and return its id (i.e. its index in the `components` vector).
    fn push_component(&mut self, component: impl Into<pavex_bp_schema::Component>) -> usize {
        let id = self.schema.components.len();
        self.schema.components.push(component.into());
        id
    }
}

/// Methods to serialize and deserialize a [`Blueprint`].
/// These are used to pass the blueprint data to Pavex's CLI.
impl Blueprint {
    /// Serialize the [`Blueprint`] to a file in RON format.
    ///
    /// The file is only written to disk if the content of the blueprint has changed.
    pub fn persist(&self, filepath: &std::path::Path) -> Result<(), anyhow::Error> {
        let config = ron::ser::PrettyConfig::new();
        let contents = ron::ser::to_string_pretty(&self.schema, config)?;
        persist_if_changed::persist_if_changed(filepath, contents.as_bytes())?;
        Ok(())
    }

    /// Read a RON-encoded [`Blueprint`] from a file.
    pub fn load(filepath: &std::path::Path) -> Result<Self, anyhow::Error> {
        let file = fs_err::OpenOptions::new().read(true).open(filepath)?;
        let value: BlueprintSchema = ron::de::from_reader(&file)?;
        Ok(Self { schema: value })
    }
}
