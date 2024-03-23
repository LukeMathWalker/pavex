use crate::blueprint::conversions::{
    cloning2cloning, lifecycle2lifecycle, method_guard2method_guard,
    raw_callable2registered_callable,
};
use crate::blueprint::error_observer::RegisteredErrorObserver;
use crate::blueprint::router::RegisteredFallback;
use pavex_bp_schema::{
    Blueprint as BlueprintSchema, Constructor, Fallback, NestedBlueprint, PostProcessingMiddleware,
    PreProcessingMiddleware, Route, WrappingMiddleware,
};
use pavex_reflection::Location;

use super::constructor::{Lifecycle, RegisteredConstructor};
use super::middleware::{
    RegisteredPostProcessingMiddleware, RegisteredPreProcessingMiddleware,
    RegisteredWrappingMiddleware,
};
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
    pub fn singleton(&mut self, callable: RawCallable) -> RegisteredConstructor {
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
    pub fn request_scoped(&mut self, callable: RawCallable) -> RegisteredConstructor {
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
    pub fn transient(&mut self, callable: RawCallable) -> RegisteredConstructor {
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
    /// use pavex::{f, blueprint::Blueprint, response::Response};
    /// use pavex_tracing::{
    ///     RootSpan,
    ///     fields::{http_response_status_code, HTTP_RESPONSE_STATUS_CODE}
    /// };
    ///
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
    ///     bp.post_process(f!(crate::response_logger));
    ///     // [...]
    ///     bp
    /// }
    /// ```
    #[doc(alias = "middleware")]
    #[doc(alias = "postprocess")]
    pub fn post_process(&mut self, callable: RawCallable) -> RegisteredPostProcessingMiddleware {
        let registered = PostProcessingMiddleware {
            middleware: raw_callable2registered_callable(callable),
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
    /// # Example: access control
    ///
    /// ```rust
    /// use pavex::{f, blueprint::Blueprint, response::Response};
    /// use pavex::middleware::Processing;
    /// use pavex::http::header::USER_AGENT;
    /// use pavex::request::RequestHead;
    ///
    /// /// Reject requests without a `User-Agent` header.
    /// pub fn reject_anonymous(request_head: &RequestHead) -> Processing
    /// {
    ///     if request_head.headers.get(USER_AGENT).is_none() {
    ///         Processing::EarlyReturn(Response::unauthorized())
    ///     } else {
    ///         Processing::Continue
    ///     }
    /// }
    ///
    /// pub fn api() -> Blueprint {
    ///     let mut bp = Blueprint::new();
    ///     // Register the pre-processing middleware against the blueprint.
    ///     bp.pre_process(f!(crate::reject_anonymous));
    ///     // [...]
    ///     bp
    /// }
    /// ```
    #[doc(alias = "middleware")]
    #[doc(alias = "preprocess")]
    pub fn pre_process(&mut self, callable: RawCallable) -> RegisteredPreProcessingMiddleware {
        let registered = PreProcessingMiddleware {
            middleware: raw_callable2registered_callable(callable),
            error_handler: None,
        };
        let component_id = self.push_component(registered);
        RegisteredPreProcessingMiddleware {
            blueprint: &mut self.schema,
            component_id,
        }
    }

    pub(super) fn register_post_processing_middleware(
        &mut self,
        mw: super::middleware::PostProcessingMiddleware,
    ) -> RegisteredPostProcessingMiddleware {
        let mw = PostProcessingMiddleware {
            middleware: mw.callable,
            error_handler: mw.error_handler,
        };
        let component_id = self.push_component(mw);
        RegisteredPostProcessingMiddleware {
            component_id,
            blueprint: &mut self.schema,
        }
    }

    pub(super) fn register_pre_processing_middleware(
        &mut self,
        mw: super::middleware::PreProcessingMiddleware,
    ) -> RegisteredPreProcessingMiddleware {
        let mw = PostProcessingMiddleware {
            middleware: mw.callable,
            error_handler: mw.error_handler,
        };
        let component_id = self.push_component(mw);
        RegisteredPreProcessingMiddleware {
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
    ///
    /// pub fn error_logger(e: &pavex::Error) {
    ///     tracing::error!(
    ///         error.msg = %e,
    ///         error.details = ?e,
    ///         "An error occurred while handling a request"
    ///     );
    /// }
    ///
    /// # fn main() {
    /// let mut bp = Blueprint::new();
    /// bp.error_observer(f!(crate::error_logger));
    /// # }
    /// ```
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
