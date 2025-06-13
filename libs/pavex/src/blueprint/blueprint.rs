use crate::blueprint::conversions::{
    cloning2cloning, lifecycle2lifecycle, method_guard2method_guard, raw_identifiers2callable,
    raw_identifiers2type,
};
use crate::blueprint::error_observer::RegisteredErrorObserver;
use crate::blueprint::prebuilt::RegisteredPrebuiltType;
use crate::blueprint::router::RegisteredFallback;
use pavex_bp_schema::{
    Blueprint as BlueprintSchema, ConfigType, Constructor, Fallback, Import, NestedBlueprint,
    PostProcessingMiddleware, PreProcessingMiddleware, PrebuiltType, Route, RoutesImport,
    WrappingMiddleware,
};
use pavex_reflection::Location;

use super::config::RegisteredConfigType;
use super::constructor::{Lifecycle, RegisteredConstructor};
use super::conversions::{coordinates2coordinates, created_at2created_at, sources2sources};
use super::error_handler::RegisteredErrorHandler;
use super::import::RegisteredImport;
use super::middleware::{
    RegisteredPostProcessingMiddleware, RegisteredPreProcessingMiddleware,
    RegisteredWrappingMiddleware,
};
use super::nesting::NestingConditions;
use super::raw::{
    RawErrorHandler, RawPostProcessingMiddleware, RawPreProcessingMiddleware, RawWrappingMiddleware,
};
use super::reflection::{RawIdentifiers, Sources, WithLocation};
use super::router::{MethodGuard, RegisteredRoute, RegisteredRoutes};

/// The starting point for building an application with Pavex.
///
/// # Guide
///
/// Check out the ["Project structure"](https://pavex.dev/docs/guide/project_structure) section of
/// Pavex's guide for more details on the role of [`Blueprint`] in Pavex applications.
///
/// # Overview
///
/// A blueprint defines the runtime behaviour of your application.
/// It keeps track of:
///
/// - route handlers, registered via [`Blueprint::route`]
/// - constructors, registered via [`Blueprint::constructor`]
/// - wrapping middlewares, registered via [`Blueprint::wrap`]
/// - fallback handlers, registered via [`Blueprint::fallback`]
///
/// You can also choose to decompose your overall application into smaller sub-components,
/// taking advantage of [`Blueprint::nest`], [`Blueprint::prefix`] and [`Blueprint::domain`].
///
/// The information encoded in a blueprint can be serialized via [`Blueprint::persist`] and passed
/// as input to Pavex's CLI to generate the application's server SDK.
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
    /// use pavex::blueprint::{from, Blueprint};
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
    /// use pavex::blueprint::{from, Blueprint};
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
    /// use pavex::blueprint::{from, Blueprint};
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
    /// use pavex::blueprint::{from, Blueprint};
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
    pub fn import(&mut self, sources: WithLocation<Sources>) -> RegisteredImport {
        let WithLocation {
            value: sources,
            created_at,
        } = sources;
        self.register_import(Import {
            sources: sources2sources(sources),
            created_at: created_at2created_at(created_at),
            registered_at: Location::caller(),
        })
    }

    #[track_caller]
    /// Register all the request handlers defined in the target modules.
    ///
    /// Components that have been annotated with Pavex's macros (e.g. `#[pavex::get]`) aren't automatically
    /// added to the router of your application.\
    /// They need to be explicitly imported using one or more invocations of this method.
    ///
    /// # Guide
    ///
    /// Check out the ["Routing"](https://pavex.dev/docs/guide/routing) section of Pavex's guide
    /// for a thorough introduction to routing in Pavex applications.
    ///
    /// # All local request handlers
    ///
    /// Use `crate` as source to register all the request handlers defined in the current crate:
    ///
    /// ```rust
    /// use pavex::blueprint::{from, Blueprint};
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
    /// use pavex::blueprint::{from, Blueprint};
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
    /// You can register request handlers defined in one of your dependencies using the same mechanism:
    ///
    /// ```rust
    /// use pavex::blueprint::{from, Blueprint};
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
    /// You can import all request handlers defined in the current crate and its direct dependencies using the wildcard source, `*`:
    ///
    /// ```rust
    /// use pavex::blueprint::{from, Blueprint};
    ///
    /// # fn main() {
    /// let mut bp = Blueprint::new();
    /// bp.routes(from![*]);
    /// # }
    /// ```
    ///
    /// This is generally discouraged.
    pub fn routes(&mut self, sources: WithLocation<Sources>) -> RegisteredRoutes {
        let WithLocation {
            value: sources,
            created_at,
        } = sources;
        let import = RoutesImport {
            sources: sources2sources(sources),
            created_at: created_at2created_at(created_at),
            registered_at: Location::caller(),
        };
        let component_id = self.push_component(import);
        RegisteredRoutes {
            blueprint: &mut self.schema,
            component_id,
        }
    }

    pub(crate) fn register_import(&mut self, import: pavex_bp_schema::Import) -> RegisteredImport {
        let component_id = self.push_component(import);
        RegisteredImport {
            blueprint: &mut self.schema,
            component_id,
        }
    }

    #[track_caller]
    /// Register a request handler to be invoked when an incoming request matches the specified route.
    ///
    /// If a request handler has already been registered for the same route, it will be overwritten.
    ///
    /// # Guide
    ///
    /// Check out the ["Routing"](https://pavex.dev/docs/guide/routing) section of Pavex's guide
    /// for a thorough introduction to routing in Pavex applications.
    ///
    /// # Example
    ///
    /// ```rust
    /// use pavex::{f, blueprint::{Blueprint, router::GET}};
    /// use pavex::{request::RequestHead, response::Response};
    ///
    /// fn my_handler(request_head: &RequestHead) -> Response {
    ///     // [...]
    ///     # todo!()
    /// }
    ///
    /// # fn main() {
    /// let mut bp = Blueprint::new();
    /// bp.route(GET, "/path", f!(crate::my_handler));
    /// # }
    /// ```
    ///
    /// [`router`]: crate::blueprint::router
    /// [`PathParams`]: struct@crate::request::path::PathParams
    pub fn route(
        &mut self,
        method_guard: MethodGuard,
        path: &str,
        callable: WithLocation<RawIdentifiers>,
    ) -> RegisteredRoute {
        let registered_route = Route {
            path: path.to_owned(),
            method_guard: method_guard2method_guard(method_guard),
            request_handler: raw_identifiers2callable(callable),
            error_handler: None,
        };
        let component_id = self.push_component(registered_route);
        RegisteredRoute {
            blueprint: &mut self.schema,
            component_id,
        }
    }

    pub(super) fn register_route(&mut self, r: super::router::Route) -> RegisteredRoute {
        let r = Route {
            path: r.path,
            method_guard: method_guard2method_guard(r.method_guard),
            error_handler: r.error_handler,
            request_handler: r.callable,
        };
        let component_id = self.push_component(r);
        RegisteredRoute {
            component_id,
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
    pub fn prebuilt(&mut self, type_: WithLocation<RawIdentifiers>) -> RegisteredPrebuiltType {
        let registered = PrebuiltType {
            input: raw_identifiers2type(type_),
            cloning_strategy: None,
        };
        let component_id = self.push_component(registered);
        RegisteredPrebuiltType {
            blueprint: &mut self.schema,
            component_id,
        }
    }

    pub(super) fn register_prebuilt_type(
        &mut self,
        i: super::prebuilt::PrebuiltType,
    ) -> RegisteredPrebuiltType {
        let i = PrebuiltType {
            input: i.type_,
            cloning_strategy: None,
        };
        let component_id = self.push_component(i);
        RegisteredPrebuiltType {
            component_id,
            blueprint: &mut self.schema,
        }
    }

    #[track_caller]
    /// Add a new type to the application's configuration.
    ///
    /// It adds a new field to the generate `ApplicationConfig` struct.
    /// Its name matches the key you provided.
    /// Its type matches the one you specified via the [`t!`](crate::t) macro.
    ///
    /// # Required traits
    ///
    /// Configuration types *must* implement `Debug`, `Clone` and `serde::Deserialize`.
    ///
    /// # Guide
    ///
    /// Check out the ["Configuration"](https://pavex.dev/docs/guide/configuration)
    /// section of Pavex's guide for a thorough introduction to Pavex's configuration system.
    pub fn config(
        &mut self,
        key: &str,
        type_: WithLocation<RawIdentifiers>,
    ) -> RegisteredConfigType {
        let registered = pavex_bp_schema::ConfigType {
            input: raw_identifiers2type(type_),
            key: key.to_owned(),
            cloning_strategy: None,
            default_if_missing: None,
            include_if_unused: None,
        };
        let component_id = self.push_component(registered);
        RegisteredConfigType {
            blueprint: &mut self.schema,
            component_id,
        }
    }

    pub(super) fn register_config_type(
        &mut self,
        i: super::config::ConfigType,
    ) -> RegisteredConfigType {
        let i = ConfigType {
            input: i.type_,
            key: i.key,
            cloning_strategy: i.cloning_strategy.map(cloning2cloning),
            default_if_missing: i.default_if_missing,
            include_if_unused: i.include_if_unused,
        };
        let component_id = self.push_component(i);
        RegisteredConfigType {
            component_id,
            blueprint: &mut self.schema,
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
    /// # Example
    ///
    /// ```rust
    /// use pavex::f;
    /// use pavex::blueprint::{Blueprint, constructor::Lifecycle};
    /// # struct LogLevel;
    /// # struct Logger;
    ///
    /// fn logger(log_level: LogLevel) -> Logger {
    ///     // [...]
    ///     # todo!()
    /// }
    ///
    /// # fn main() {
    /// let mut bp = Blueprint::new();
    /// bp.constructor(f!(crate::logger), Lifecycle::Transient);
    /// # }
    /// ```
    pub fn constructor(
        &mut self,
        callable: WithLocation<RawIdentifiers>,
        lifecycle: Lifecycle,
    ) -> RegisteredConstructor {
        let registered_constructor = Constructor {
            constructor: raw_identifiers2callable(callable),
            lifecycle: lifecycle2lifecycle(lifecycle),
            cloning_strategy: None,
            error_handler: None,
            lints: Default::default(),
        };
        let component_id = self.push_component(registered_constructor);
        RegisteredConstructor {
            component_id,
            blueprint: &mut self.schema,
        }
    }

    #[track_caller]
    /// Register a constructor with a [singleton lifecycle][Lifecycle::Singleton].
    ///
    /// It's a shorthand for [`Blueprint::constructor`]—refer to its documentation for
    /// more information on dependency injection in Pavex.
    ///
    /// # Example
    ///
    /// ```rust
    /// use pavex::f;
    /// use pavex::blueprint::Blueprint;
    /// # struct LogLevel;
    /// # struct Logger;
    ///
    /// fn logger(log_level: LogLevel) -> Logger {
    ///     // [...]
    ///     # todo!()
    /// }
    ///
    /// # fn main() {
    /// let mut bp = Blueprint::new();
    /// bp.singleton(f!(crate::logger));
    /// // ^ is equivalent to:
    /// // bp.constructor(f!(crate::logger), Lifecycle::Singleton));
    /// # }
    /// ```
    pub fn singleton(&mut self, callable: WithLocation<RawIdentifiers>) -> RegisteredConstructor {
        self.constructor(callable, Lifecycle::Singleton)
    }

    #[track_caller]
    /// Register a constructor with a [request-scoped lifecycle][Lifecycle::RequestScoped].
    ///
    /// It's a shorthand for [`Blueprint::constructor`]—refer to its documentation for
    /// more information on dependency injection in Pavex.
    ///
    /// # Example
    ///
    /// ```rust
    /// use pavex::f;
    /// use pavex::blueprint::Blueprint;
    /// # struct LogLevel;
    /// # struct Logger;
    ///
    /// fn logger(log_level: LogLevel) -> Logger {
    ///     // [...]
    ///     # todo!()
    /// }
    ///
    /// # fn main() {
    /// let mut bp = Blueprint::new();
    /// bp.request_scoped(f!(crate::logger));
    /// // ^ is equivalent to:
    /// // bp.constructor(f!(crate::logger), Lifecycle::RequestScoped));
    /// # }
    /// ```
    pub fn request_scoped(
        &mut self,
        callable: WithLocation<RawIdentifiers>,
    ) -> RegisteredConstructor {
        self.constructor(callable, Lifecycle::RequestScoped)
    }

    #[track_caller]
    /// Register a constructor with a [transient lifecycle][Lifecycle::Transient].
    ///
    /// It's a shorthand for [`Blueprint::constructor`]—refer to its documentation for
    /// more information on dependency injection in Pavex.
    ///
    /// # Example
    ///
    /// ```rust
    /// use pavex::f;
    /// use pavex::blueprint::Blueprint;
    /// # struct LogLevel;
    /// # struct Logger;
    ///
    /// fn logger(log_level: LogLevel) -> Logger {
    ///     // [...]
    ///     # todo!()
    /// }
    ///
    /// # fn main() {
    /// let mut bp = Blueprint::new();
    /// bp.transient(f!(crate::logger));
    /// // ^ is equivalent to:
    /// // bp.constructor(f!(crate::logger), Lifecycle::Transient));
    /// # }
    /// ```
    pub fn transient(&mut self, callable: WithLocation<RawIdentifiers>) -> RegisteredConstructor {
        self.constructor(callable, Lifecycle::Transient)
    }

    pub(super) fn register_constructor(
        &mut self,
        constructor: super::constructor::Constructor,
    ) -> RegisteredConstructor {
        let constructor = Constructor {
            constructor: constructor.callable,
            lifecycle: lifecycle2lifecycle(constructor.lifecycle),
            cloning_strategy: constructor.cloning_strategy.map(cloning2cloning),
            error_handler: constructor.error_handler,
            lints: constructor.lints,
        };
        let component_id = self.push_component(constructor);
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
    /// # Example: a timeout wrapper
    ///
    /// ```rust
    /// use pavex::{blueprint::Blueprint, middleware::Next, response::Response};
    /// use std::future::{IntoFuture, Future};
    /// use std::time::Duration;
    /// use tokio::time::{timeout, error::Elapsed};
    ///
    /// #[pavex::wrap]
    /// pub async fn timeout_wrapper<C>(next: Next<C>) -> Result<Response, Elapsed>
    /// where
    ///     C: Future<Output = Response>
    /// {
    ///     timeout(Duration::from_secs(2), next.into_future()).await
    /// }
    ///
    /// pub fn api() -> Blueprint {
    ///     let mut bp = Blueprint::new();
    ///     // Register the wrapping middleware against the blueprint.
    ///     bp.wrap(TIMEOUT_WRAPPER);
    ///     // [...]
    ///     bp
    /// }
    /// ```
    #[doc(alias = "middleware")]
    pub fn wrap(&mut self, m: RawWrappingMiddleware) -> RegisteredWrappingMiddleware {
        let registered = WrappingMiddleware {
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
    /// # Example: a logging middleware
    ///
    /// ```rust
    /// use pavex::{blueprint::Blueprint, response::Response};
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
    ///
    /// pub fn api() -> Blueprint {
    ///     let mut bp = Blueprint::new();
    ///     // Register the post-processing middleware against the blueprint.
    ///     bp.post_process(RESPONSE_LOGGER);
    ///     // [...]
    ///     bp
    /// }
    /// ```
    #[doc(alias = "middleware")]
    #[doc(alias = "postprocess")]
    pub fn post_process(
        &mut self,
        m: RawPostProcessingMiddleware,
    ) -> RegisteredPostProcessingMiddleware {
        let registered = PostProcessingMiddleware {
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
    /// # Example: path normalization
    ///
    /// ```rust
    /// use pavex::{blueprint::Blueprint, response::Response};
    /// use pavex::middleware::Processing;
    /// use pavex::http::{HeaderValue, header::LOCATION};
    /// use pavex::request::RequestHead;
    ///
    /// /// If the request path ends with a `/`,
    /// /// redirect to the same path without the trailing `/`.
    /// #[pavex::pre_process]
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
    ///
    /// pub fn api() -> Blueprint {
    ///     let mut bp = Blueprint::new();
    ///     // Register the pre-processing middleware against the blueprint.
    ///     bp.pre_process(REDIRECT_TO_NORMALIZED);
    ///     // [...]
    ///     bp
    /// }
    /// ```
    #[doc(alias = "middleware")]
    #[doc(alias = "preprocess")]
    pub fn pre_process(
        &mut self,
        m: RawPreProcessingMiddleware,
    ) -> RegisteredPreProcessingMiddleware {
        let registered = PreProcessingMiddleware {
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
        self.push_component(NestedBlueprint {
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
    /// use pavex::f;
    /// use pavex::blueprint::{Blueprint, router::GET};
    ///
    /// fn app() -> Blueprint {
    ///     let mut bp = Blueprint::new();
    ///     // Adding `/api` as common prefix here
    ///     bp.prefix("/api").nest(api_bp());
    ///     bp
    /// }
    ///
    /// fn api_bp() -> Blueprint {
    ///     let mut bp = Blueprint::new();
    ///     // This will match `GET` requests to `/api/path`.
    ///     bp.route(GET, "/path", f!(crate::handler));
    ///     bp
    /// }
    /// # pub fn handler() {}
    /// ```
    ///
    /// You can also add a (sub)domain constraint, in addition to the common prefix:
    ///
    /// ```rust
    /// use pavex::blueprint::{Blueprint, router::GET};
    /// use pavex::f;
    ///
    /// fn app() -> Blueprint {
    ///    let mut bp = Blueprint::new();
    ///    bp.prefix("/v1").domain("api.mybusiness.com").nest(api_bp());
    ///    bp
    /// }
    ///
    /// fn api_bp() -> Blueprint {
    ///    let mut bp = Blueprint::new();
    ///   // This will match `GET` requests to `api.mybusiness.com/v1/path`.
    ///   bp.route(GET, "/path", f!(crate::handler));
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
    /// use pavex::f;
    /// use pavex::blueprint::{Blueprint, router::GET};
    ///
    /// fn app() -> Blueprint {
    ///     let mut bp = Blueprint::new();
    ///     bp.prefix("/api").nest(api_bp());
    ///     bp
    /// }
    ///
    /// fn api_bp() -> Blueprint {
    ///     let mut bp = Blueprint::new();
    ///     // This will match `GET` requests to `/api//path`.
    ///     bp.route(GET, "//path", f!(crate::handler));
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
    /// use pavex::blueprint::Blueprint;
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
    /// use pavex::blueprint::{Blueprint, router::GET};
    /// use pavex::f;
    ///
    /// fn app() -> Blueprint {
    ///    let mut bp = Blueprint::new();
    ///    bp.prefix("/v1").domain("api.mybusiness.com").nest(api_bp());
    ///    bp
    /// }
    ///
    /// fn api_bp() -> Blueprint {
    ///    let mut bp = Blueprint::new();
    ///   // This will match `GET` requests to `api.mybusiness.com/v1/path`.
    ///   bp.route(GET, "/path", f!(crate::handler));
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
    /// use pavex::{f, blueprint::{Blueprint, router::GET}};
    /// use pavex::response::Response;
    ///
    /// fn handler() -> Response {
    ///     // [...]
    ///     # todo!()
    /// }
    /// fn fallback_handler() -> Response {
    ///     // [...]
    ///     # todo!()
    /// }
    ///
    /// # fn main() {
    /// let mut bp = Blueprint::new();
    /// bp.route(GET, "/path", f!(crate::handler));
    /// // The fallback handler will be invoked for all the requests that don't match `/path`.
    /// // E.g. `GET /home`, `POST /home`, `GET /home/123`, etc.
    /// bp.fallback(f!(crate::fallback_handler));
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
    /// use pavex::{f, blueprint::{Blueprint, router::GET}};
    /// use pavex::response::Response;
    ///
    /// # fn route_handler() -> Response { todo!() }
    /// # fn home_handler() -> Response { todo!() }
    /// fn fallback_handler() -> Response {
    ///     // [...]
    ///     # todo!()
    /// }
    ///
    /// # fn main() {
    /// let mut bp = Blueprint::new();
    /// bp.route(GET, "/home", f!(crate::home_handler));
    /// bp.nest({
    ///     let mut bp = Blueprint::new();
    ///     bp.route(GET, "/route", f!(crate::route_handler));
    ///     bp.fallback(f!(crate::fallback_handler));
    ///     bp
    /// });
    /// # }
    /// ```
    ///
    /// In the example above, `crate::fallback_handler` will be invoked for incoming `POST /route`
    /// requests: the path matches the path of a route registered against the nested blueprint
    /// (`GET /route`), but the method guard doesn't (`POST` vs `GET`).
    /// If the incoming requests don't have `/route` as their path instead (e.g. `GET /street`
    /// or `GET /route/123`), they will be handled by the fallback registered against the **parent**
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
    /// use pavex::{f, blueprint::{Blueprint, router::GET}};
    /// use pavex::response::Response;
    ///
    /// # fn route_handler() -> Response { todo!() }
    /// # fn home_handler() -> Response { todo!() }
    /// fn fallback_handler() -> Response {
    ///     // [...]
    ///     # todo!()
    /// }
    ///
    /// # fn main() {
    /// let mut bp = Blueprint::new();
    /// bp.route(GET, "/home", f!(crate::home_handler));
    /// bp.prefix("/route").nest({
    ///     let mut bp = Blueprint::new();
    ///     bp.route(GET, "/", f!(crate::route_handler));
    ///     bp.fallback(f!(crate::fallback_handler));
    ///     bp
    /// });
    /// # }
    /// ```
    ///
    /// In the example above, `crate::fallback_handler` will be invoked for both `POST /route`
    /// **and** `POST /route/123` requests: the path of the latter doesn't match the path of the only
    /// route registered against the nested blueprint (`GET /route`), but it starts with the
    /// prefix of the nested blueprint (`/route`).
    ///
    /// [`Response`]: crate::response::Response
    pub fn fallback(&mut self, callable: WithLocation<RawIdentifiers>) -> RegisteredFallback {
        let registered = Fallback {
            request_handler: raw_identifiers2callable(callable),
            error_handler: None,
        };
        let component_id = self.push_component(registered);
        RegisteredFallback {
            blueprint: &mut self.schema,
            component_id,
        }
    }

    pub(super) fn register_fallback(&mut self, f: super::router::Fallback) -> RegisteredFallback {
        let f = Fallback {
            request_handler: f.callable,
            error_handler: f.error_handler,
        };
        let component_id = self.push_component(f);
        RegisteredFallback {
            component_id,
            blueprint: &mut self.schema,
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
    /// # Example
    ///
    /// ```rust
    /// use pavex::f;
    /// use pavex::blueprint::Blueprint;
    /// use tracing_log_error::log_error;
    ///
    /// pub fn error_logger(e: &pavex::Error) {
    ///     log_error!(e, "An error occurred while handling a request");
    /// }
    ///
    /// # fn main() {
    /// let mut bp = Blueprint::new();
    /// bp.error_observer(f!(crate::error_logger));
    /// # }
    /// ```
    pub fn error_observer(
        &mut self,
        callable: WithLocation<RawIdentifiers>,
    ) -> RegisteredErrorObserver {
        let registered = pavex_bp_schema::ErrorObserver {
            error_observer: raw_identifiers2callable(callable),
        };
        self.push_component(registered);
        RegisteredErrorObserver {
            blueprint: &mut self.schema,
        }
    }

    pub(super) fn register_error_observer(
        &mut self,
        eo: super::error_observer::ErrorObserver,
    ) -> RegisteredErrorObserver {
        let eo = pavex_bp_schema::ErrorObserver {
            error_observer: eo.callable,
        };
        self.push_component(eo);
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
    pub fn error_handler(&mut self, m: RawErrorHandler) -> RegisteredErrorHandler {
        let registered = pavex_bp_schema::ErrorHandler {
            coordinates: coordinates2coordinates(m.coordinates),
            registered_at: Location::caller(),
        };
        self.push_component(registered);
        RegisteredErrorHandler {
            blueprint: &mut self.schema,
        }
    }

    /// Register a component and return its id (i.e. its index in the `components` vector).
    pub fn push_component(&mut self, component: impl Into<pavex_bp_schema::Component>) -> usize {
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
