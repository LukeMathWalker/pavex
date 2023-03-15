use std::fmt::{Display, Formatter};

use indexmap::{IndexMap, IndexSet};

use crate::callable::{RawCallable, RawCallableIdentifiers};
use crate::router::MethodGuard;

#[derive(Default, serde::Serialize, serde::Deserialize)]
/// A blueprint for the runtime behaviour of your application.
///
/// `Blueprint` captures three types of information:
///
/// - route handlers, via [`Blueprint::route`].
/// - constructors, via [`Blueprint::constructor`].
/// - error handlers, via [`Constructor::error_handler`].
///
/// This information is then serialized via [`Blueprint::persist`] and passed as input to
/// `pavex_cli` to generate the application's source code.
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
    pub error_handler_locations: IndexMap<RawCallableIdentifiers, Location>,
    /// - Keys: [`RawCallableIdentifiers`] of a constructor.
    /// - Values: a [`Location`] pointing at the corresponding invocation of
    /// [`Blueprint::constructor`].
    pub constructor_locations: IndexMap<RawCallableIdentifiers, Location>,
    /// All registered routes, in the order they were registered.
    pub routes: Vec<RegisteredRoute>,
}

#[derive(serde::Serialize, serde::Deserialize)]
/// A route registered against a [`Blueprint`] via [`Blueprint::route`].
pub struct RegisteredRoute {
    /// The path of the route.
    pub path: String,
    /// The HTTP method guard for the route.
    pub method_guard: MethodGuard,
    /// The callable in charge of processing incoming requests for this route.
    pub request_handler: RegisteredCallable,
    /// The callable in charge of processing errors returned by the request handler, if any.
    pub error_handler: Option<RegisteredCallable>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct RegisteredCallable {
    pub callable: RawCallableIdentifiers,
    pub location: Location,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
/// How many times should a constructor be invoked?
pub enum Lifecycle {
    /// The constructor for a `Singleton` type is invoked at most once.
    ///
    /// As a consequence, there is at most one instance of `Singleton` types,
    /// stored inside the server's global state.  
    Singleton,
    /// The constructor for a `RequestScoped` type is invoked at most once for every incoming request.
    ///
    /// As a consequence, there is at most one instance of `RequestScoped` types for every incoming
    /// request.
    RequestScoped,
    /// The constructor for a `Transient` type is invoked every single time an instance of the type
    /// is required.
    ///
    /// As a consequence, there is can be **multiple** instances of `Transient` types for every
    /// incoming request.
    Transient,
}

impl Display for Lifecycle {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Lifecycle::Singleton => "singleton",
            Lifecycle::RequestScoped => "request-scoped",
            Lifecycle::Transient => "transient",
        };
        write!(f, "{s}")
    }
}

impl Blueprint {
    /// Create a new [`Blueprint`].
    pub fn new() -> Self {
        Default::default()
    }

    #[track_caller]
    /// Register a constructor.
    ///
    /// ```rust
    /// use pavex_builder::{Blueprint, f, Lifecycle};
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
    /// The value of the route parameter `home_id` can then be retrieved from the request handler
    /// (or any other constructor that has access to the request):
    ///
    /// ```rust
    /// use pavex_runtime::extract::route::RouteParams;
    ///
    /// #[derive(serde::Deserialize)]
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
    /// Check out the [`extract::path`] module in `pavex_runtime` for more information
    /// on how to use route parameters.
    ///
    /// [`router`]: crate::router
    /// [`extract::path`]: pavex_runtime::extract::route
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

    /// Serialize the blueprint data to a file in RON format.
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

#[derive(Clone, Debug, Eq, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
/// A set of coordinates to identify a precise spot in a source file.
///
/// # Implementation Notes
///
/// `Location` is an owned version of [`std::panic::Location`].  
/// You can build a `Location` instance starting from a [`std::panic::Location`]:
///
/// ```rust
/// use pavex_builder::Location;
///
/// let location: Location = std::panic::Location::caller().into();
/// ```
pub struct Location {
    /// The line number.
    ///
    /// Lines are 1-indexed (i.e. the first line is numbered as 1, not 0).
    pub line: u32,
    /// The column number.
    ///
    /// Columns are 1-indexed (i.e. the first column is numbered as 1, not 0).
    pub column: u32,
    /// The name of the source file.
    ///
    /// Check out [`std::panic::Location::file`] for more details.
    pub file: String,
}

impl<'a> From<&'a std::panic::Location<'a>> for Location {
    fn from(l: &'a std::panic::Location<'a>) -> Self {
        Self {
            line: l.line(),
            column: l.column(),
            file: l.file().into(),
        }
    }
}

/// The type returned by [`Blueprint::route`].
///
/// It allows you to further configure the behaviour of the registered route.
pub struct Route<'a> {
    #[allow(dead_code)]
    blueprint: &'a mut Blueprint,
    route_id: usize,
}

impl<'a> Route<'a> {
    #[track_caller]
    /// Register an error handler.
    ///
    /// Error handlers convert the error type returned by your request handler into an HTTP response.
    ///
    /// Error handlers CANNOT consume the error type, they must take a reference to the
    /// error as input.  
    /// Error handlers can have additional input parameters alongside the error, as long as there
    /// are constructors registered for those parameter types.
    ///
    /// ```rust
    /// use pavex_builder::{Blueprint, f, router::GET};
    /// use pavex_runtime::{response::Response, hyper::body::Body};
    /// # struct LogLevel;
    /// # struct RuntimeError;
    /// # struct ConfigurationError;
    ///
    /// fn request_handler() -> Result<Response, RuntimeError> {
    ///     // [...]
    ///     # todo!()
    /// }
    ///
    /// fn error_to_response(error: &ConfigurationError, log_level: LogLevel) -> Response {
    ///     // [...]
    ///     # todo!()
    /// }
    ///
    /// # fn main() {
    /// let mut bp = Blueprint::new();
    /// bp.route(GET, "/home", f!(crate::request_handler))
    ///     .error_handler(f!(crate::error_to_response));
    /// # }
    /// ```
    ///
    /// If an error handler has already been registered for the same error type, it will be
    /// overwritten.
    ///
    /// ## Common Errors
    ///
    /// `pavex_cli` will fail to generate the runtime code for your application if you register
    /// an error handler for an infallible request handler (i.e. a request handler that doesn't
    /// return a `Result`).
    pub fn error_handler(self, error_handler: RawCallable) -> Self {
        let callable_identifiers = RawCallableIdentifiers::new(error_handler.import_path);
        let callable = RegisteredCallable {
            callable: callable_identifiers,
            location: std::panic::Location::caller().into(),
        };
        self.blueprint.routes[self.route_id].error_handler = Some(callable);
        self
    }
}

/// The type returned by [`Blueprint::constructor`].
///
/// It allows you to further configure the behaviour of the registered constructor.
pub struct Constructor<'a> {
    blueprint: &'a mut Blueprint,
    constructor_identifiers: RawCallableIdentifiers,
}

impl<'a> Constructor<'a> {
    #[track_caller]
    /// Register an error handler.
    ///
    /// Error handlers convert the error type returned by your constructor into an HTTP response.
    ///
    /// Error handlers CANNOT consume the error type, they must take a reference to the
    /// error as input.  
    /// Error handlers can have additional input parameters alongside the error, as long as there
    /// are constructors registered for those parameter types.
    ///
    /// ```rust
    /// use pavex_builder::{Blueprint, f, Lifecycle};
    /// use pavex_runtime::{response::Response, hyper::body::Body};
    /// # struct LogLevel;
    /// # struct Logger;
    /// # struct ConfigurationError;
    ///
    /// fn logger() -> Result<Logger, ConfigurationError> {
    ///     // [...]
    ///     # todo!()
    /// }
    ///
    /// fn error_to_response(error: &ConfigurationError, log_level: LogLevel) -> Response {
    ///     // [...]
    ///     # todo!()
    /// }
    ///
    /// # fn main() {
    /// let mut bp = Blueprint::new();
    /// bp.constructor(f!(crate::logger), Lifecycle::Transient)
    ///     .error_handler(f!(crate::error_to_response));
    /// # }
    /// ```
    ///
    /// If an error handler has already been registered for the same error type, it will be
    /// overwritten.
    ///
    /// ## Common Errors
    ///
    /// `pavex_cli` will fail to generate the runtime code for your application if you register
    /// an error handler for an infallible constructor (i.e. a constructor that doesn't return
    /// a `Result`).
    pub fn error_handler(self, handler: RawCallable) -> Self {
        let callable_identifiers = RawCallableIdentifiers::new(handler.import_path);
        self.blueprint.error_handler_locations.insert(
            self.constructor_identifiers.clone(),
            std::panic::Location::caller().into(),
        );
        self.blueprint
            .constructors_error_handlers
            .insert(self.constructor_identifiers.clone(), callable_identifiers);
        self
    }
}
