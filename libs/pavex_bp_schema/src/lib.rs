//! The schema used by Pavex to serialize and deserialize blueprints.
//!
//! There are no guarantees that this schema will remain stable across Pavex versions:
//! it is considered (for the time being) an internal implementation detail of Pavex's reflection system.
pub use pavex_reflection::{Location, RawIdentifiers, RegisteredAt};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fmt::Formatter;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct Blueprint {
    /// The location where the `Blueprint` was created.
    pub creation_location: Location,
    /// All registered components, in the order they were registered.
    pub components: Vec<Component>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub enum Component {
    Constructor(Constructor),
    WrappingMiddleware(WrappingMiddleware),
    PostProcessingMiddleware(PostProcessingMiddleware),
    PreProcessingMiddleware(PreProcessingMiddleware),
    Route(Route),
    FallbackRequestHandler(Fallback),
    NestedBlueprint(NestedBlueprint),
    ErrorObserver(ErrorObserver),
    PrebuiltType(PrebuiltType),
}

impl From<PrebuiltType> for Component {
    fn from(i: PrebuiltType) -> Self {
        Self::PrebuiltType(i)
    }
}

impl From<Constructor> for Component {
    fn from(c: Constructor) -> Self {
        Self::Constructor(c)
    }
}

impl From<WrappingMiddleware> for Component {
    fn from(m: WrappingMiddleware) -> Self {
        Self::WrappingMiddleware(m)
    }
}

impl From<PostProcessingMiddleware> for Component {
    fn from(m: PostProcessingMiddleware) -> Self {
        Self::PostProcessingMiddleware(m)
    }
}

impl From<PreProcessingMiddleware> for Component {
    fn from(m: PreProcessingMiddleware) -> Self {
        Self::PreProcessingMiddleware(m)
    }
}

impl From<Route> for Component {
    fn from(r: Route) -> Self {
        Self::Route(r)
    }
}

impl From<Fallback> for Component {
    fn from(f: Fallback) -> Self {
        Self::FallbackRequestHandler(f)
    }
}

impl From<NestedBlueprint> for Component {
    fn from(b: NestedBlueprint) -> Self {
        Self::NestedBlueprint(b)
    }
}

impl From<ErrorObserver> for Component {
    fn from(e: ErrorObserver) -> Self {
        Self::ErrorObserver(e)
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
/// A route registered against a `Blueprint` via `Blueprint::route`.
pub struct Route {
    /// The path of the route.
    pub path: String,
    /// The HTTP method guard for the route.
    pub method_guard: MethodGuard,
    /// The callable in charge of processing incoming requests for this route.
    pub request_handler: Callable,
    /// The callable in charge of processing errors returned by the request handler, if any.
    pub error_handler: Option<Callable>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
/// A request handler registered against a `Blueprint` via `Blueprint::fallback` to
/// process requests that don't match any of the registered routes.
pub struct Fallback {
    /// The callable in charge of processing incoming requests.
    pub request_handler: Callable,
    /// The callable in charge of processing errors returned by the request handler, if any.
    pub error_handler: Option<Callable>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
/// An error observer registered against a `Blueprint` via `Blueprint::error_observer` to
/// intercept unhandled errors.
pub struct ErrorObserver {
    /// The callable in charge of processing unhandled errors.
    pub error_observer: Callable,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
/// A type registered against a `Blueprint` via `Blueprint::prebuilt` to
/// be added as an input parameter to `build_application_state`.
pub struct PrebuiltType {
    /// The type.
    pub input: Type,
    /// The strategy dictating when the prebuilt type can be cloned.
    pub cloning_strategy: Option<CloningStrategy>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
/// A constructor registered against a `Blueprint` via `Blueprint::constructor`.
pub struct Constructor {
    /// The callable in charge of constructing the desired type.
    pub constructor: Callable,
    /// The lifecycle of the constructed type.
    pub lifecycle: Lifecycle,
    /// The strategy dictating when the constructed type can be cloned.
    pub cloning_strategy: Option<CloningStrategy>,
    /// The callable in charge of processing errors returned by this constructor, if any.
    pub error_handler: Option<Callable>,
    /// Lint settings for this constructor.
    pub lints: BTreeMap<Lint, LintSetting>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
/// A middleware registered against a `Blueprint` via `Blueprint::wrap`.
pub struct WrappingMiddleware {
    /// The callable that executes the middleware's logic.
    pub middleware: Callable,
    /// The callable in charge of processing errors returned by this middleware, if any.
    pub error_handler: Option<Callable>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
/// A middleware registered against a `Blueprint` via `Blueprint::post_process`.
pub struct PostProcessingMiddleware {
    /// The callable that executes the middleware's logic.
    pub middleware: Callable,
    /// The callable in charge of processing errors returned by this middleware, if any.
    pub error_handler: Option<Callable>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
/// A middleware registered against a `Blueprint` via `Blueprint::pre_process`.
pub struct PreProcessingMiddleware {
    /// The callable that executes the middleware's logic.
    pub middleware: Callable,
    /// The callable in charge of processing errors returned by this middleware, if any.
    pub error_handler: Option<Callable>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
/// A "callable" registered against a `Blueprint`—either a free function or a method,
/// used as a request handler, error handler or constructor.
pub struct Callable {
    /// Metadata that uniquely identifies the callable.
    pub callable: RawIdentifiers,
    /// The location where the callable was registered against the `Blueprint`.
    pub location: Location,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
/// A type (enum or struct) registered against a `Blueprint`.
pub struct Type {
    /// Metadata that uniquely identifies the type.
    pub type_: RawIdentifiers,
    /// The location where the type was registered against the `Blueprint`.
    pub location: Location,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
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

#[derive(
    Debug, Clone, Copy, Eq, Ord, PartialOrd, PartialEq, Hash, serde::Serialize, serde::Deserialize,
)]
#[non_exhaustive]
/// Common mistakes and antipatterns that Pavex
/// tries to catch when analysing your [`Blueprint`].  
pub enum Lint {
    /// You registered a component that's never used in the generated
    /// server SDK code.
    Unused,
}

#[derive(
    Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum LintSetting {
    Ignore,
    Enforce,
}
