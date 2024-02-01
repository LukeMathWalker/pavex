use crate::blueprint::conversions::{
    cloning2cloning, lifecycle2lifecycle, method_guard2method_guard,
    raw_callable2registered_callable,
};
use crate::blueprint::error_observer::RegisteredErrorObserver;
use crate::blueprint::router::RegisteredFallback;
use pavex_bp_schema::{
    Blueprint as BlueprintSchema, Constructor, Fallback, NestedBlueprint, Route, WrappingMiddleware,
};
use pavex_reflection::Location;

use super::constructor::{Lifecycle, RegisteredConstructor};
use super::middleware::RegisteredWrappingMiddleware;
use super::reflection::RawCallable;
use super::router::{MethodGuard, RegisteredRoute};

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
/// taking advantage of [`Blueprint::nest`] and [`Blueprint::nest_at`].
///
/// The information encoded in a blueprint can be serialized via [`Blueprint::persist`] and passed
/// as input to Pavex's CLI to generate the application's server SDK.
pub struct Blueprint {
    schema: BlueprintSchema,
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
        callable: RawCallable,
    ) -> RegisteredRoute {
        let registered_route = Route {
            path: path.to_owned(),
            method_guard: method_guard2method_guard(method_guard),
            request_handler: raw_callable2registered_callable(callable),
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
        callable: RawCallable,
        lifecycle: Lifecycle,
    ) -> RegisteredConstructor {
        let registered_constructor = Constructor {
            constructor: raw_callable2registered_callable(callable),
            lifecycle: lifecycle2lifecycle(lifecycle),
            cloning_strategy: None,
            error_handler: None,
        };
        let component_id = self.push_component(registered_constructor);
        RegisteredConstructor {
            component_id,
            blueprint: &mut self.schema,
        }
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
    /// A wrapping middleware is invoked before the request handler and it is given
    /// the opportunity to *wrap* the execution of the rest of the request processing
    /// pipeline, including the request handler itself.
    ///
    /// It is primarily useful for functionality that requires access to the [`Future`]
    /// representing the rest of the request processing pipeline, such as:
    ///
    /// - structured logging (e.g. attaching a `tracing` span to the request execution);
    /// - timeouts;
    /// - metric timers;
    /// - etc.
    ///
    /// # Example: a timeout wrapper
    ///
    /// ```rust
    /// use pavex::{f, blueprint::Blueprint, middleware::Next, response::Response};
    /// use std::future::{IntoFuture, Future};
    /// use std::time::Duration;
    /// use tokio::time::{timeout, error::Elapsed};
    ///
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
    ///     bp.wrap(f!(crate::timeout_wrapper));
    ///     // [...]
    ///     bp
    /// }
    /// ```
    ///
    /// # Signature
    ///
    /// A wrapping middleware is an asynchronous function (or a method) that takes [`Next`]
    /// as input and returns a [`Response`], either directly (if infallible) or wrapped in a
    /// [`Result`] (if fallible).
    ///
    /// [`Next`] represents the rest of the request processing pipeline, including the request
    /// handler itself.  
    /// It can be awaited directly or converted into a [`Future`] via the
    /// [`into_future`](std::future::IntoFuture) method.
    ///
    /// ```rust
    /// use pavex::{middleware::Next, response::Response};
    /// use std::{future::{IntoFuture, Future}, time::Duration};
    /// use tokio::time::{timeout, error::Elapsed};
    /// use tracing::Instrument;
    ///
    /// // This is an infallible wrapping middleware. It returns a `Response` directly.
    /// pub async fn logging_wrapper<C>(next: Next<C>) -> Response
    /// where
    ///     C: Future<Output = Response>
    /// {
    ///     let span = tracing::info_span!("Incoming request");
    ///     next.into_future().instrument(span).await
    /// }
    ///
    /// // This is a fallible wrapping middleware.
    /// // It returns a `Result<Response, Elapsed>`.
    /// pub async fn timeout_wrapper<C>(next: Next<C>) -> Result<Response, Elapsed>
    /// where
    ///     C: Future<Output = Response>
    /// {
    ///     timeout(Duration::from_secs(1), next.into_future()).await
    /// }
    /// ```
    ///
    /// ## Dependency injection
    ///
    /// Wrapping middlewares can take advantage of dependency injection, like any
    /// other component.  
    /// You list what you want to inject as function parameters (in _addition_ to [`Next`])
    /// and Pavex will inject them for you in the generated code:
    ///
    /// ```rust
    /// use pavex::{
    ///     blueprint::{Blueprint, constructor::Lifecycle},
    ///     f, middleware::Next, response::Response
    /// };
    /// use std::{future::{IntoFuture, Future}, time::Duration};
    /// use tokio::time::{timeout, error::Elapsed};
    ///
    /// #[derive(Copy, Clone)]
    /// pub struct TimeoutConfig {
    ///     request_timeout: Duration
    /// }
    ///
    /// pub async fn timeout_wrapper<C>(
    ///     next: Next<C>,
    ///     // This parameter will be injected by the framework.
    ///     config: TimeoutConfig
    /// ) -> Result<Response, Elapsed>
    /// where
    ///     C: IntoFuture<Output = Response>
    /// {
    ///     timeout(config.request_timeout, next.into_future()).await
    /// }
    ///
    /// pub fn api() -> Blueprint {
    ///     let mut bp = Blueprint::new();
    ///     // We need to register a constructor for the dependencies
    ///     // that we want to inject
    ///     bp.constructor(f!(crate::timeout_config), Lifecycle::RequestScoped);
    ///     bp.wrap(f!(crate::timeout_wrapper));
    ///     // [...]
    ///     bp
    /// }
    /// ```
    ///
    /// # Execution order
    ///
    /// Wrapping middlewares are invoked in the order they are registered.
    ///
    /// ```rust
    /// use pavex::{f, blueprint::{Blueprint, router::GET}};
    /// # use pavex::{request::RequestHead, response::Response, middleware::Next};
    /// # use std::future::Future;
    /// # pub fn first<C: Future<Output = Response>>(next: Next<C>) -> Response { todo!() }
    /// # pub fn second<C: Future<Output = Response>>(next: Next<C>) -> Response { todo!() }
    /// # fn main() {
    /// let mut bp = Blueprint::new();
    /// bp.wrap(f!(crate::first));
    /// bp.wrap(f!(crate::second));
    /// bp.route(GET, "/home", f!(crate::handler));
    /// # }
    /// ```
    ///
    /// `first` will be invoked before `second`, which is in turn invoked before the
    /// request handler.  
    /// Or, in other words:
    ///
    /// - `second` is invoked when `first` calls `.await` on its `Next` input
    /// - the request handler is invoked when `second` calls `.await` on its `Next` input
    ///
    /// ## Nesting
    ///
    /// If a blueprint is nested under another blueprint, the wrapping middlewares registered
    /// against the parent blueprint will be invoked before the wrapping middlewares registered
    /// against the nested blueprint.
    ///
    /// [`Next`]: crate::middleware::Next
    /// [`Response`]: crate::response::Response
    /// [`Future`]: std::future::Future
    #[doc(alias = "middleware")]
    pub fn wrap(&mut self, callable: RawCallable) -> RegisteredWrappingMiddleware {
        let registered = WrappingMiddleware {
            middleware: raw_callable2registered_callable(callable),
            error_handler: None,
        };
        let component_id = self.push_component(registered);
        RegisteredWrappingMiddleware {
            blueprint: &mut self.schema,
            component_id,
        }
    }

    pub(super) fn register_wrapping_middleware(
        &mut self,
        mw: super::middleware::WrappingMiddleware,
    ) -> RegisteredWrappingMiddleware {
        let mw = WrappingMiddleware {
            middleware: mw.callable,
            error_handler: mw.error_handler,
        };
        let component_id = self.push_component(mw);
        RegisteredWrappingMiddleware {
            component_id,
            blueprint: &mut self.schema,
        }
    }

    #[track_caller]
    /// Nest a [`Blueprint`] under the current [`Blueprint`] (the parent), adding a common prefix to all the new routes.  
    ///
    /// # Routes
    ///
    /// `prefix` will be prepended to all the routes coming from the nested blueprint.  
    /// `prefix` must be non-empty and it must start with a `/`.  
    /// If you don't want to add a common prefix, check out [`Blueprint::nest`].
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
    ///     bp.nest_at("/api", api_bp());
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
    ///
    /// # Constructors
    ///
    /// Constructors registered against the parent blueprint will be available to the nested
    /// blueprint—they are **inherited**.  
    /// Constructors registered against the nested blueprint will **not** be available to other
    /// sibling blueprints that are nested under the same parent—they are **private**.
    ///
    /// Check out the example below to better understand the implications of nesting blueprints.
    ///
    /// ## Visibility
    ///
    /// ```rust
    /// use pavex::f;
    /// use pavex::blueprint::{Blueprint, router::GET};
    /// use pavex::blueprint::constructor::Lifecycle;
    ///
    /// fn app() -> Blueprint {
    ///     let mut bp = Blueprint::new();
    ///     bp.constructor(f!(crate::db_connection_pool), Lifecycle::Singleton);
    ///     bp.nest(home_bp());
    ///     bp.nest(user_bp());
    ///     bp
    /// }
    ///
    /// /// All property-related routes and constructors.
    /// fn home_bp() -> Blueprint {
    ///     let mut bp = Blueprint::new();
    ///     bp.route(GET, "/home", f!(crate::v1::get_home));
    ///     bp
    /// }
    ///
    /// /// All user-related routes and constructors.
    /// fn user_bp() -> Blueprint {
    ///     let mut bp = Blueprint::new();
    ///     bp.constructor(f!(crate::user::get_session), Lifecycle::RequestScoped);
    ///     bp.route(GET, "/user", f!(crate::user::get_user));
    ///     bp
    /// }
    /// # pub fn db_connection_pool() {}
    /// # mod home { pub fn get_home() {} }
    /// # mod user {
    /// #     pub fn get_user() {}
    /// #     pub fn get_session() {}
    /// # }
    /// ```
    ///
    /// This example registers two routes:
    /// - `GET /home`
    /// - `GET /user`
    ///
    /// It also registers two constructors:
    /// - `crate::user::get_session`, for `Session`;
    /// - `crate::db_connection_pool`, for `ConnectionPool`.
    ///
    /// Since we are **nesting** the `user_bp` blueprint, the `get_session` constructor will only
    /// be available to the routes declared in the `user_bp` blueprint.  
    /// If a route declared in `home_bp` tries to inject a `Session`, Pavex will report an error
    /// at compile-time, complaining that there is no registered constructor for `Session`.
    /// In other words, all constructors declared against the `user_bp` blueprint are **private**
    /// and **isolated** from the rest of the application.
    ///
    /// The `db_connection_pool` constructor, instead, is declared against the parent blueprint
    /// and will therefore be available to all routes declared in `home_bp` and `user_bp`—i.e.
    /// nested blueprints **inherit** all the constructors declared against their parent(s).
    ///
    /// ## Precedence
    ///
    /// If a constructor is declared against both the parent and one of its nested blueprints, the one
    /// declared against the nested blueprint takes precedence.
    ///
    /// ```rust
    /// use pavex::f;
    /// use pavex::blueprint::{Blueprint, router::GET};
    /// use pavex::blueprint::constructor::Lifecycle;
    ///
    /// fn app() -> Blueprint {
    ///     let mut bp = Blueprint::new();
    ///     // This constructor is registered against the root blueprint and it's visible
    ///     // to all nested blueprints.
    ///     bp.constructor(f!(crate::global::get_session), Lifecycle::RequestScoped);
    ///     bp.nest(user_bp());
    ///     // [..]
    ///     bp
    /// }
    ///
    /// fn user_bp() -> Blueprint {
    ///     let mut bp = Blueprint::new();
    ///     // It can be overridden by a constructor for the same type registered
    ///     // against a nested blueprint.
    ///     // All routes in `user_bp` will use `user::get_session` instead of `global::get_session`.
    ///     bp.constructor(f!(crate::user::get_session), Lifecycle::RequestScoped);
    ///     // [...]
    ///     bp
    /// }
    /// # mod global { pub fn get_session() {} }
    /// # mod user {
    /// #     pub fn get_user() {}
    /// #     pub fn get_session() {}
    /// # }
    /// ```
    ///
    /// ## Singletons
    ///
    /// There is one exception to the precedence rule: constructors for singletons (i.e.
    /// using [`Lifecycle::Singleton`]).  
    /// Pavex guarantees that there will be only one instance of a singleton type for the entire
    /// lifecycle of the application. What should happen if two different constructors are registered for
    /// the same `Singleton` type by two nested blueprints that share the same parent?  
    /// We can't honor both constructors without ending up with two different instances of the same
    /// type, which would violate the singleton contract.  
    ///
    /// It goes one step further! Even if those two constructors are identical, what is the expected
    /// behaviour? Does the user expect the same singleton instance to be injected in both blueprints?
    /// Or does the user expect two different singleton instances to be injected in each nested blueprint?
    ///
    /// To avoid this ambiguity, Pavex takes a conservative approach: a singleton constructor
    /// must be registered **exactly once** for each type.  
    /// If multiple nested blueprints need access to the singleton, the constructor must be
    /// registered against a common parent blueprint—the root blueprint, if necessary.
    pub fn nest_at(&mut self, prefix: &str, blueprint: Blueprint) {
        self.push_component(NestedBlueprint {
            blueprint: blueprint.schema,
            path_prefix: Some(prefix.into()),
            nesting_location: Location::caller(),
        });
    }

    #[track_caller]
    /// Nest a [`Blueprint`] under the current [`Blueprint`] (the parent), without adding a common prefix to all the new routes.  
    ///
    /// Check out [`Blueprint::nest_at`] for more details.
    pub fn nest(&mut self, blueprint: Blueprint) {
        self.push_component(NestedBlueprint {
            blueprint: blueprint.schema,
            path_prefix: None,
            nesting_location: Location::caller(),
        });
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
    /// If your application takes advantage of [nesting](Blueprint::nest_at), you can register
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
    /// bp.nest_at("/route", {
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
    pub fn fallback(&mut self, callable: RawCallable) -> RegisteredFallback {
        let registered = Fallback {
            request_handler: raw_callable2registered_callable(callable),
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
    pub fn error_observer(&mut self, callable: RawCallable) -> RegisteredErrorObserver {
        let registered = pavex_bp_schema::ErrorObserver {
            error_observer: raw_callable2registered_callable(callable),
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
    pub fn persist(&self, filepath: &std::path::Path) -> Result<(), anyhow::Error> {
        let mut file = fs_err::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(filepath)?;
        let config = ron::ser::PrettyConfig::new();
        ron::ser::to_writer_pretty(&mut file, &self.schema, config)?;
        Ok(())
    }

    /// Read a RON-encoded [`Blueprint`] from a file.
    pub fn load(filepath: &std::path::Path) -> Result<Self, anyhow::Error> {
        let file = fs_err::OpenOptions::new().read(true).open(filepath)?;
        let value: BlueprintSchema = ron::de::from_reader(&file)?;
        Ok(Self { schema: value })
    }
}
