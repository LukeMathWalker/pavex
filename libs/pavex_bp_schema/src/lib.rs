//! The schema used by Pavex to serialize and deserialize blueprints.
//!
//! There are no guarantees that this schema will remain stable across Pavex versions:
//! it is considered (for the time being) an internal implementation detail of Pavex's reflection system.
pub use pavex_reflection::{CreatedAt, Location, RawIdentifiers};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fmt::Formatter;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
/// The blueprint for a Pavex application.
pub struct Blueprint {
    /// The location where the `Blueprint` was created.
    pub creation_location: Location,
    /// All registered components, in the order they were registered.
    pub components: Vec<Component>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
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
    ConfigType(ConfigType),
    Import(Import),
}

impl From<PrebuiltType> for Component {
    fn from(i: PrebuiltType) -> Self {
        Self::PrebuiltType(i)
    }
}

impl From<ConfigType> for Component {
    fn from(c: ConfigType) -> Self {
        Self::ConfigType(c)
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

impl From<Import> for Component {
    fn from(i: Import) -> Self {
        Self::Import(i)
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Import {
    pub sources: Sources,
    pub created_at: CreatedAt,
    pub registered_at: Location,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
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

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
/// A request handler registered against a `Blueprint` via `Blueprint::fallback` to
/// process requests that don't match any of the registered routes.
pub struct Fallback {
    /// The callable in charge of processing incoming requests.
    pub request_handler: Callable,
    /// The callable in charge of processing errors returned by the request handler, if any.
    pub error_handler: Option<Callable>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
/// An error observer registered against a `Blueprint` via `Blueprint::error_observer` to
/// intercept unhandled errors.
pub struct ErrorObserver {
    /// The callable in charge of processing unhandled errors.
    pub error_observer: Callable,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
/// A type registered against a `Blueprint` via `Blueprint::prebuilt` to
/// be added as an input parameter to `ApplicationState::new`.
pub struct PrebuiltType {
    /// The type.
    pub input: Type,
    /// The strategy dictating when the prebuilt type can be cloned.
    pub cloning_strategy: Option<CloningStrategy>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
/// A type registered against a `Blueprint` via `Blueprint::config` to
/// become part of the overall configuration for the application.
pub struct ConfigType {
    /// The type.
    pub input: Type,
    /// The field name.
    pub key: String,
    /// The strategy dictating when the config type can be cloned.
    pub cloning_strategy: Option<CloningStrategy>,
    /// Whether to use `Default::default` to generate default configuration
    /// values if the user hasn't specified any.
    pub default_if_missing: Option<bool>,
    /// Whether to include the config type as a field in the generated
    /// configuration struct even if it was never injected.
    pub include_if_unused: Option<bool>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
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

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
/// A middleware registered against a `Blueprint` via `Blueprint::wrap`.
pub struct WrappingMiddleware {
    /// The callable that executes the middleware's logic.
    pub middleware: Callable,
    /// The callable in charge of processing errors returned by this middleware, if any.
    pub error_handler: Option<Callable>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
/// A middleware registered against a `Blueprint` via `Blueprint::post_process`.
pub struct PostProcessingMiddleware {
    /// The callable that executes the middleware's logic.
    pub middleware: Callable,
    /// The callable in charge of processing errors returned by this middleware, if any.
    pub error_handler: Option<Callable>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
/// A middleware registered against a `Blueprint` via `Blueprint::pre_process`.
pub struct PreProcessingMiddleware {
    /// The callable that executes the middleware's logic.
    pub middleware: Callable,
    /// The callable in charge of processing errors returned by this middleware, if any.
    pub error_handler: Option<Callable>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
/// A "callable" registered against a `Blueprint`—either a free function or a method,
/// used as a request handler, error handler or constructor.
pub struct Callable {
    /// Metadata that uniquely identifies the callable.
    pub callable: RawIdentifiers,
    /// The location where the callable was registered against the `Blueprint`.
    pub registered_at: Location,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
/// A type (enum or struct) registered against a `Blueprint`.
pub struct Type {
    /// Metadata that uniquely identifies the type.
    pub type_: RawIdentifiers,
    /// The location where the type was registered against the `Blueprint`.
    pub registered_at: Location,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
/// A `Blueprint` that has been nested inside another `Blueprint` via `Blueprint::nest` or
/// `Blueprint::nest_at`.
pub struct NestedBlueprint {
    /// The nested `Blueprint`.
    pub blueprint: Blueprint,
    /// The path prefix that will prepended to all routes registered against the nested
    /// `Blueprint`.
    /// If `None`, the routes coming from the nested `Blueprint` will be registered as-they-are.
    pub path_prefix: Option<PathPrefix>,
    /// If `Some`, only requests whose `Host` header matches this value will be forwarded to the
    /// routes registered against this nested `Blueprint`.
    pub domain: Option<Domain>,
    /// The location where the `Blueprint` was nested under its parent `Blueprint`.
    pub nested_at: Location,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
/// A path modifier for a nested [`Blueprint`].
pub struct PathPrefix {
    /// The path prefix to prepend to all routes registered against the nested [`Blueprint`].
    pub path_prefix: String,
    /// The location where the path prefix was registered.
    pub registered_at: Location,
}

/// A domain routing constraint.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Domain {
    /// The domain to match.
    pub domain: String,
    /// The location where the domain constraint was registered.
    pub registered_at: Location,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
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
#[serde(rename_all = "snake_case")]
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
#[serde(rename_all = "snake_case")]
pub enum MethodGuard {
    Any,
    Some(BTreeSet<String>),
}

#[derive(
    Debug, Clone, Copy, Eq, Ord, PartialOrd, PartialEq, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
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
#[serde(rename_all = "snake_case")]
pub enum LintSetting {
    Ignore,
    Enforce,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
/// A collection of modules to be scanned for components.
pub enum Sources {
    /// Use all valid sources: modules from the current crate and all its direct dependencies.
    All,
    /// Use only the specified modules as sources.
    ///
    /// Each module can be either from the current crate or from one of its direct dependencies.
    Some(Vec<String>),
}
