use indexmap::{IndexMap, IndexSet};

use crate::constructor::{Constructor, Lifecycle};
use crate::internals::{NestedBlueprint, RegisteredCallable, RegisteredRoute};
use crate::reflection::{Location, RawCallable, RawCallableIdentifiers};
use crate::router::{MethodGuard, Route};

#[derive(Default, serde::Serialize, serde::Deserialize)]
/// The starting point for building an application with `pavex`.
///
/// A blueprint defines the runtime behaviour of your application.  
/// It captures three types of information:
///
/// - route handlers, via [`Blueprint::route`].
/// - constructors, via [`Blueprint::constructor`].
/// - error handlers, via [`Constructor::error_handler`].
///
/// This information is then serialized via [`Blueprint::persist`] and passed as input to
/// `pavex`'s CLI to generate the application's source code.
///
/// [`Constructor::error_handler`]: Constructor::error_handler
pub struct Blueprint {
    /// The set of registered constructors.
    pub constructors: IndexSet<RawCallableIdentifiers>,
    /// - Keys: [`RawCallableIdentifiers`] of a **fallible** constructor.
    /// - Values: [`RawCallableIdentifiers`] of an error handler for the error type returned by
    /// the constructor.
    pub constructors_error_handlers: IndexMap<RawCallableIdentifiers, RawCallableIdentifiers>,
    /// - Keys: [`RawCallableIdentifiers`] of a constructor.
    /// - Values: the [`Lifecycle`] for the type returned by the constructor.
    pub component_lifecycles: IndexMap<RawCallableIdentifiers, Lifecycle>,
    /// - Keys: [`RawCallableIdentifiers`] of the fallible constructor.
    /// - Values: a [`Location`] pointing at the corresponding invocation of
    /// [`Constructor::error_handler`].
    ///
    /// [`Constructor::error_handler`]: Constructor::error_handler
    pub error_handler_locations: IndexMap<RawCallableIdentifiers, Location>,
    /// - Keys: [`RawCallableIdentifiers`] of a constructor.
    /// - Values: a [`Location`] pointing at the corresponding invocation of
    /// [`Blueprint::constructor`].
    pub constructor_locations: IndexMap<RawCallableIdentifiers, Location>,
    /// All registered routes, in the order they were registered.
    pub routes: Vec<RegisteredRoute>,
    /// All blueprints nested under this one, in the order they were nested.
    pub nested_blueprints: Vec<NestedBlueprint>,
}

impl Blueprint {
    /// Create a new [`Blueprint`].
    pub fn new() -> Self {
        Default::default()
    }

    #[track_caller]
    /// Register a request handler to be invoked when an incoming request matches the specified route.
    ///
    /// If a request handler has already been registered for the same route, it will be overwritten.
    ///
    /// # Routing: an introduction
    ///
    /// ## Simple routes
    ///
    /// The simplest route is a combination of a single HTTP method, a path and a request handler:
    ///
    /// ```rust
    /// use pavex_builder::{Blueprint, f, router::GET};
    /// use pavex_runtime::{http::Request, hyper::Body, response::Response};
    ///
    /// fn my_handler(request: Request<Body>) -> Response {
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
    /// You can use the constants exported in the [`router`] module to specify one of the well-known
    /// HTTP methods:
    ///
    /// ```rust
    /// use pavex_builder::{Blueprint, f, router::{GET, POST, PUT, DELETE, PATCH}};
    /// # use pavex_runtime::{http::Request, hyper::Body, response::Response};
    /// # fn my_handler(request: Request<Body>) -> Response { todo!() }
    /// # fn main() {
    /// # let mut bp = Blueprint::new();
    ///
    /// bp.route(GET, "/path", f!(crate::my_handler));
    /// bp.route(POST, "/path", f!(crate::my_handler));
    /// bp.route(PUT, "/path", f!(crate::my_handler));
    /// bp.route(DELETE, "/path", f!(crate::my_handler));
    /// bp.route(PATCH, "/path", f!(crate::my_handler));
    /// // ...and a few more!
    /// # }
    /// ```
    ///
    /// ## Matching multiple HTTP methods
    ///
    /// It can also be useful to register a request handler that handles multiple HTTP methods
    /// for the same path:
    ///
    /// ```rust
    /// use pavex_builder::{Blueprint, f, router::{MethodGuard, POST, PATCH}};
    /// use pavex_runtime::http::Method;
    /// # use pavex_runtime::{http::Request, hyper::Body, response::Response};
    /// # fn my_handler(request: Request<Body>) -> Response { todo!() }
    /// # fn main() {
    /// # let mut bp = Blueprint::new();
    ///
    /// // `crate::my_handler` will be used to handle both `PATCH` and `POST` requests to `/path`
    /// bp.route(
    ///     MethodGuard::new([Method::PATCH, Method::POST]),
    ///     "/path",
    ///     f!(crate::my_handler)
    /// );
    /// # }
    /// ```
    ///
    /// Last but not least, you can register a route that matches a request **regardless** of
    /// the HTTP method being used:
    ///
    /// ```rust
    /// use pavex_builder::{Blueprint, f, router::ANY};
    /// # use pavex_runtime::{http::Request, hyper::Body, response::Response};
    /// # fn my_handler(request: Request<Body>) -> Response { todo!() }
    /// # fn main() {
    /// # let mut bp = Blueprint::new();
    ///
    /// // This will match **all** incoming requests to `/path`, regardless of their HTTP method.
    /// // `GET`, `POST`, `PUT`... anything goes!
    /// bp.route(ANY, "/path", f!(crate::my_handler));
    /// # }
    /// ```
    ///
    /// ## Route parameters
    ///
    /// Your route paths can include **route parameters**—a way to bind the
    /// value of a path segment from an incoming request and make it available to your request
    /// handler.
    ///
    /// Let's look at an example—a route with a single route parameter, `home_id`:
    ///
    /// ```rust
    /// use pavex_builder::{Blueprint, f, router::GET};
    /// # use pavex_runtime::{http::Request, hyper::Body, response::Response};
    /// # fn get_home(request: Request<Body>) -> Response { todo!() }
    /// # fn main() {
    /// # let mut bp = Blueprint::new();
    ///
    /// // This route will match `GET` requests to `/home/123` and `/home/456`, but not `/home`.
    /// bp.route(GET, "/home/:home_id", f!(crate::get_home));
    /// # }
    /// ```
    ///
    /// Route parameters are path segments prefixed with a colon (`:`)—`:home_id` in the example.  
    /// The value of the route parameter `home_id` can then be retrieved from the request handler
    /// (or any other constructor that has access to the request):
    ///
    /// ```rust
    /// use pavex_runtime::extract::route::RouteParams;
    ///
    /// #[RouteParams]
    /// struct HomeRouteParams {
    ///     // The name of the field must match the name of the route parameter
    ///     // used in the template we passed to `bp.route`.
    ///     home_id: u32,
    /// }
    ///
    /// // The `RouteParams` extractor will deserialize the route parameters into the
    /// // type you specified—`HomeRouteParams` in this case.
    /// fn get_home(params: &RouteParams<HomeRouteParams>) -> String {
    ///     format!("Fetching the home with id {}", params.0.home_id)
    /// }
    /// ```
    ///
    /// `pavex` supports **catch-all** parameters as well: they start with `*` and match
    /// everything after the `/`.
    ///
    /// ```rust
    /// use pavex_builder::{Blueprint, f, router::GET};
    /// # use pavex_runtime::{http::Request, hyper::Body, response::Response};
    /// # fn get_town(request: Request<Body>) -> Response { todo!() }
    /// # fn main() {
    /// # let mut bp = Blueprint::new();
    ///
    /// // This route will match, for example, `GET` requests to:
    /// // - `/town/123`, with `town_info=123`
    /// // - `/town/456/street/123`, with `town_info=456/street/123`
    /// //
    /// // It won't match a GET request to `/town/`, `town_info` cannot be empty.
    /// bp.route(GET, "/town/:*town_info", f!(crate::get_town));
    /// # }
    /// ```
    ///
    /// There can be at most one catch-all parameter in a route, and
    /// it **must** be at the end of the route template
    ///
    /// Check out [`RouteParams`] in `pavex_runtime` for more information
    /// on how to extract and work with route parameters.
    ///
    /// [`router`]: crate::router
    /// [`RouteParams`]: struct@pavex_runtime::extract::route::RouteParams
    pub fn route(&mut self, method_guard: MethodGuard, path: &str, callable: RawCallable) -> Route {
        let registered_route = RegisteredRoute {
            path: path.to_owned(),
            method_guard,
            request_handler: RegisteredCallable {
                callable: RawCallableIdentifiers::new(callable.import_path),
                location: std::panic::Location::caller().into(),
            },
            error_handler: None,
        };
        let route_id = self.routes.len();
        self.routes.push(registered_route);
        Route {
            blueprint: self,
            route_id,
        }
    }

    #[track_caller]
    /// Register a constructor.
    ///
    /// ```rust
    /// use pavex_builder::{Blueprint, f, constructor::Lifecycle};
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
    ///
    /// If a constructor for the same type has already been registered, it will be overwritten.
    pub fn constructor(&mut self, callable: RawCallable, lifecycle: Lifecycle) -> Constructor {
        let callable_identifiers = RawCallableIdentifiers::new(callable.import_path);
        let location = std::panic::Location::caller();
        self.constructor_locations
            .entry(callable_identifiers.clone())
            .or_insert_with(|| location.into());
        self.component_lifecycles
            .insert(callable_identifiers.clone(), lifecycle);
        self.constructors.insert(callable_identifiers.clone());
        Constructor {
            constructor_identifiers: callable_identifiers,
            blueprint: self,
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
    /// use pavex_builder::{Blueprint, f, router::GET};
    /// use pavex_builder::constructor::Lifecycle;
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
    /// If a route declared in `home_bp` tries to inject a `Session`, `pavex` will report an error
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
    /// use pavex_builder::{Blueprint, f, router::GET};
    /// use pavex_builder::constructor::Lifecycle;
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
    /// `pavex` guarantees that there will be only one instance of a singleton type for the entire
    /// lifecycle of the application. What should happen if two different constructors are registered for
    /// the same `Singleton` type by two nested blueprints that share the same parent?  
    /// We can't honor both constructors without ending up with two different instances of the same
    /// type, which would violate the singleton contract.  
    ///
    /// It goes one step further! Even if those two constructors are identical, what is the expected
    /// behaviour? Does the user expect the same singleton instance to be injected in both blueprints?
    /// Or does the user expect two different singleton instances to be injected in each nested blueprint?
    ///
    /// To avoid this ambiguity, `pavex` takes a conservative approach: a singleton constructor
    /// must be registered **exactly once** for each type.  
    /// If multiple nested blueprints need access to the singleton, the constructor must be
    /// registered against a common parent blueprint—the root blueprint, if necessary.
    pub fn nest_at(&mut self, prefix: &str, blueprint: Blueprint) {
        self.nested_blueprints.push(NestedBlueprint {
            blueprint,
            path_prefix: Some(prefix.into()),
            location: std::panic::Location::caller().into(),
        })
    }

    #[track_caller]
    /// Nest a [`Blueprint`] under the current [`Blueprint`] (the parent), without adding a common prefix to all the new routes.  
    ///
    /// Check out [`Blueprint::nest_at`] for more details.
    pub fn nest(&mut self, blueprint: Blueprint) {
        self.nested_blueprints.push(NestedBlueprint {
            blueprint,
            path_prefix: None,
            location: std::panic::Location::caller().into(),
        })
    }
}

/// Methods to serialize and deserialize a [`Blueprint`].  
/// These are used to pass the blueprint data to `pavex`'s CLI.
impl Blueprint {
    /// Serialize the [`Blueprint`] to a file in RON format.
    pub fn persist(&self, filepath: &std::path::Path) -> Result<(), anyhow::Error> {
        let mut file = fs_err::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(filepath)?;
        let config = ron::ser::PrettyConfig::new();
        ron::ser::to_writer_pretty(&mut file, &self, config)?;
        Ok(())
    }

    /// Read a RON-encoded [`Blueprint`] from a file.
    pub fn load(filepath: &std::path::Path) -> Result<Self, anyhow::Error> {
        let file = fs_err::OpenOptions::new().read(true).open(filepath)?;
        let value = ron::de::from_reader(&file)?;
        Ok(value)
    }
}
