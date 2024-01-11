//! The schema used by Pavex to serialize and deserialize blueprints.
pub use pavex_reflection::{Location, RawCallableIdentifiers};
use std::collections::BTreeSet;
use std::fmt;
use std::fmt::Formatter;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Blueprint {
    /// The location where the `Blueprint` was created.
    pub creation_location: Location,
    /// All registered components, in the order they were registered.
    pub components: Vec<RegisteredComponent>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub enum RegisteredComponent {
    Constructor(RegisteredConstructor),
    WrappingMiddleware(RegisteredWrappingMiddleware),
    Route(RegisteredRoute),
    FallbackRequestHandler(RegisteredFallback),
    NestedBlueprint(NestedBlueprint),
}

impl From<RegisteredConstructor> for RegisteredComponent {
    fn from(c: RegisteredConstructor) -> Self {
        Self::Constructor(c)
    }
}

impl From<RegisteredWrappingMiddleware> for RegisteredComponent {
    fn from(m: RegisteredWrappingMiddleware) -> Self {
        Self::WrappingMiddleware(m)
    }
}

impl From<RegisteredRoute> for RegisteredComponent {
    fn from(r: RegisteredRoute) -> Self {
        Self::Route(r)
    }
}

impl From<RegisteredFallback> for RegisteredComponent {
    fn from(f: RegisteredFallback) -> Self {
        Self::FallbackRequestHandler(f)
    }
}

impl From<NestedBlueprint> for RegisteredComponent {
    fn from(b: NestedBlueprint) -> Self {
        Self::NestedBlueprint(b)
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
/// A route registered against a `Blueprint` via `Blueprint::route`.
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
/// A request handler registered against a `Blueprint` via `Blueprint::fallback` to
/// process requests that don't match any of the registered routes.
pub struct RegisteredFallback {
    /// The callable in charge of processing incoming requests.
    pub request_handler: RegisteredCallable,
    /// The callable in charge of processing errors returned by the request handler, if any.
    pub error_handler: Option<RegisteredCallable>,
}

#[derive(serde::Serialize, serde::Deserialize)]
/// A constructor registered against a `Blueprint` via `Blueprint::constructor`.
pub struct RegisteredConstructor {
    /// The callable in charge of constructing the desired type.
    pub constructor: RegisteredCallable,
    /// The lifecycle of the constructed type.
    pub lifecycle: Lifecycle,
    /// The strategy dictating when the constructed type can be cloned.
    pub cloning_strategy: Option<CloningStrategy>,
    /// The callable in charge of processing errors returned by this constructor, if any.
    pub error_handler: Option<RegisteredCallable>,
}

#[derive(serde::Serialize, serde::Deserialize)]
/// A middleware registered against a `Blueprint` via `Blueprint::wrap`.
pub struct RegisteredWrappingMiddleware {
    /// The callable that executes the middleware's logic.
    pub middleware: RegisteredCallable,
    /// The callable in charge of processing errors returned by this middleware, if any.
    pub error_handler: Option<RegisteredCallable>,
}

#[derive(serde::Serialize, serde::Deserialize)]
/// A "callable" registered against a `Blueprint`—either a free function or a method,
/// used as a request handler, error handler or constructor.
pub struct RegisteredCallable {
    /// Metadata that uniquely identifies the callable.
    pub callable: RawCallableIdentifiers,
    /// The location where the callable was registered against the `Blueprint`.
    pub location: Location,
}

#[derive(serde::Serialize, serde::Deserialize)]
/// A `Blueprint` that has been nested inside another `Blueprint` via `Blueprint::nest` or
/// `Blueprint::nest_at`.
pub struct NestedBlueprint {
    /// The nested `Blueprint`.
    pub blueprint: Blueprint,
    /// The path prefix that will prepended to all routes registered against the nested
    /// `Blueprint`.
    /// If `None`, the routes coming from the nested `Blueprint` will be registered as-they-are.
    pub path_prefix: Option<String>,
    /// The location where the `Blueprint` was nested under its parent `Blueprint`.
    pub nesting_location: Location,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Lifecycle {
    Singleton,
    RequestScoped,
    Transient,
}

impl fmt::Display for Lifecycle {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Lifecycle::Singleton => write!(f, "singleton"),
            Lifecycle::RequestScoped => write!(f, "request-scoped"),
            Lifecycle::Transient => write!(f, "transient"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum CloningStrategy {
    /// Pavex will **never** try clone the output type returned by the constructor.
    NeverClone,
    /// Pavex will only clone the output type returned by this constructor if it's
    /// necessary to generate code that satisfies Rust's borrow checker.
    CloneIfNecessary,
}

#[derive(
    Debug, Clone, Hash, PartialEq, Eq, Ord, PartialOrd, serde::Serialize, serde::Deserialize,
)]
pub enum MethodGuard {
    Any,
    Some(BTreeSet<String>),
}
