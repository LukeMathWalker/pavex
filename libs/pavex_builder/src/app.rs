use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};
use std::marker::PhantomData;

use indexmap::{IndexMap, IndexSet};

use crate::callable::{RawCallable, RawCallableIdentifiers};
use crate::Callable;

#[derive(Default, serde::Serialize, serde::Deserialize)]
/// A blueprint to code-generate your application.
///
/// `AppBlueprint` captures three types of information:
///
/// - route handlers (e.g. use `my_handler` for all incoming `/home` requests).
/// - constructors (e.g. use `my_constructor` every time you need to build an instance of `MyType`).
/// - error handlers (e.g. use `error2response` every time you need to handle a `SerializationError`).
///
/// For each constructor, you must specify the [`Lifecycle`] of its output type:
///
/// - _[`Lifecycle::Singleton`]_; an instance is built once before, the application starts, and
/// re-used for all incoming requests.
/// - _[`Lifecycle::RequestScoped`]_; a new instance is built for every incoming request and re-used
/// throughout the handling of that specific request.
/// - _[`Lifecycle::Transient`]_; a new instance is built every time the type is needed, potentially
/// multiple times for each incoming request.
///
/// All this information is encoded into an `AppBlueprint` and passed as input to `pavex_cli`
/// to generate the application's source code.
pub struct AppBlueprint {
    /// The set of registered constructors.
    pub constructors: IndexSet<RawCallableIdentifiers>,
    /// The set of registered request handlers.
    pub request_handlers: IndexSet<RawCallableIdentifiers>,
    /// - Keys: [`RawCallableIdentifiers`] of a **fallible** constructor.
    /// - Values: [`RawCallableIdentifiers`] of an error handler for the error type returned by
    /// the constructor.
    pub constructor_error_handlers: IndexMap<RawCallableIdentifiers, RawCallableIdentifiers>,
    /// - Keys: [`RawCallableIdentifiers`] of a constructor.
    /// - Values: the [`Lifecycle`] for the type returned by the constructor.
    pub component_lifecycles: IndexMap<RawCallableIdentifiers, Lifecycle>,
    /// - Keys: a path (e.g. `/homes/rooms`).
    /// - Values: [`RawCallableIdentifiers`] of the request handler in charge of processing
    /// incoming requests for that path.
    pub router: BTreeMap<String, RawCallableIdentifiers>,
    /// - Keys: [`RawCallableIdentifiers`] of a request handler.
    /// - Values: a [`Location`] pointing at the corresponding invocation of
    /// [`AppBlueprint::route`].
    pub request_handler_locations: IndexMap<RawCallableIdentifiers, IndexSet<Location>>,
    /// - Keys: [`RawCallableIdentifiers`] of an error handler.
    /// - Values: a [`Location`] pointing at the corresponding invocation of
    /// [`Constructor::error_handler`].
    pub error_handler_locations: IndexMap<RawCallableIdentifiers, Location>,
    /// - Keys: [`RawCallableIdentifiers`] of a constructor.
    /// - Values: a [`Location`] pointing at the corresponding invocation of
    /// [`AppBlueprint::constructor`].
    pub constructor_locations: IndexMap<RawCallableIdentifiers, Location>,
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
        write!(f, "{}", s)
    }
}

impl AppBlueprint {
    /// Create a new [`AppBlueprint`].
    pub fn new() -> Self {
        Default::default()
    }

    #[track_caller]
    /// Register a constructor.
    ///
    /// ```rust
    /// use pavex_builder::{AppBlueprint, f, Lifecycle};
    /// # struct LogLevel;
    /// # struct Logger;
    ///
    /// fn logger(log_level: LogLevel) -> Logger {
    ///     // [...]
    ///     # todo!()
    /// }
    ///
    /// # fn main() {
    /// let mut bp = AppBlueprint::new();
    /// bp.constructor(f!(crate::logger), Lifecycle::Transient);
    /// # }
    /// ```
    ///
    /// If a constructor for the same type has already been registered, it will be overwritten.
    pub fn constructor<F, ConstructorInputs>(
        &mut self,
        callable: RawCallable<F>,
        lifecycle: Lifecycle,
    ) -> Constructor<F::Output>
    where
        F: Callable<ConstructorInputs>,
    {
        let callable_identifiers = RawCallableIdentifiers::new(callable.import_path);
        let location = std::panic::Location::caller();
        self.constructor_locations
            .entry(callable_identifiers.clone())
            .or_insert_with(|| location.into());
        self.component_lifecycles
            .insert(callable_identifiers.clone(), lifecycle);
        self.constructors.insert(callable_identifiers);
        Constructor {
            blueprint: self,
            output_type: PhantomData::<F::Output>,
        }
    }

    #[track_caller]
    /// Register a route and the corresponding request handler.
    ///
    /// If a handler has already been registered for the same route, it will be overwritten.
    pub fn route<F, HandlerInputs>(&mut self, callable: RawCallable<F>, path: &str) -> Route
    where
        F: Callable<HandlerInputs>,
    {
        let callable_identifiers = RawCallableIdentifiers::new(callable.import_path);
        self.request_handler_locations
            .entry(callable_identifiers.clone())
            .or_default()
            .insert(std::panic::Location::caller().into());
        self.router
            .insert(path.to_owned(), callable_identifiers.clone());
        self.request_handlers.insert(callable_identifiers);
        Route { blueprint: self }
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

    /// Read a RON-encoded [`AppBlueprint`] from a file.
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

/// The type returned by [`AppBlueprint::route`].
///
/// It allows you to further configure the behaviour of the registered route.
pub struct Route<'a> {
    #[allow(dead_code)]
    blueprint: &'a mut AppBlueprint,
}

/// The type returned by [`AppBlueprint::constructor`].
///
/// It allows you to further configure the behaviour of the registered constructor.
pub struct Constructor<'a, Output> {
    blueprint: &'a mut AppBlueprint,
    output_type: PhantomData<Output>,
}

impl<'a, Success, Error> Constructor<'a, Result<Success, Error>> {
    #[track_caller]
    /// Register an error handler for the error type returned by the constructor.
    ///
    /// Error handlers convert an error type into an HTTP response for the caller.
    ///
    /// Error handlers CANNOT consume the error type, they must take a reference to the
    /// error as input.  
    /// Error handlers can have additional input parameters alongside the error, as long as there
    /// are constructors registered for those parameter types.
    ///
    /// ```rust
    /// use pavex_builder::{AppBlueprint, f, Lifecycle};
    /// use pavex_runtime::{http::Response, hyper::body::Body};
    /// # struct LogLevel;
    /// # struct Logger;
    /// # struct ConfigurationError;
    ///
    /// fn logger() -> Result<Logger, ConfigurationError> {
    ///     // [...]
    ///     # todo!()
    /// }
    ///
    /// fn error_to_response(error: &ConfigurationError, log_level: LogLevel) -> Response<Body> {
    ///     // [...]
    ///     # todo!()
    /// }
    ///
    /// # fn main() {
    /// let mut bp = AppBlueprint::new();
    /// bp.constructor(f!(crate::logger), Lifecycle::Transient)
    ///     .error_handler(f!(crate::error_to_response));
    /// # }
    /// ```
    ///
    /// If an error handler has already been registered for the same error type, it will be
    /// overwritten.
    pub fn error_handler<F, HandlerInputs>(self, handler: RawCallable<F>) -> Self
    where
        F: Callable<HandlerInputs>,
    {
        let callable_identifiers = RawCallableIdentifiers::new(handler.import_path);
        let location = std::panic::Location::caller();
        self.blueprint
            .error_handler_locations
            .entry(callable_identifiers.clone())
            .or_insert_with(|| location.into());
        self.blueprint.error_handlers.insert(callable_identifiers);
        self
    }
}
