//! The schema used by Pavex to serialize and deserialize blueprints.
//!
//! There are no guarantees that this schema will remain stable across Pavex versions:
//! it is considered (for the time being) an internal implementation detail of Pavex's reflection system.
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
    ErrorHandler(ErrorHandler),
    PrebuiltType(PrebuiltType),
    ConfigType(ConfigType),
    Import(Import),
    RoutesImport(RoutesImport),
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

impl From<ErrorHandler> for Component {
    fn from(e: ErrorHandler) -> Self {
        Self::ErrorHandler(e)
    }
}

impl From<Import> for Component {
    fn from(i: Import) -> Self {
        Self::Import(i)
    }
}

impl From<RoutesImport> for Component {
    fn from(i: RoutesImport) -> Self {
        Self::RoutesImport(i)
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct RoutesImport {
    pub sources: Sources,
    /// The path of the module where this import was created (i.e. `from!` was invoked).
    ///
    /// It is used to resolve relative paths in the `from!` macro, i.e. paths starting
    /// with `super` or `self`.
    pub relative_to: String,
    pub created_at: CreatedAt,
    pub registered_at: Location,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Import {
    pub sources: Sources,
    /// The path of the module where this import was created (i.e. `from!` was invoked).
    ///
    /// It is used to resolve relative paths in the `from!` macro, i.e. paths starting
    /// with `super` or `self`.
    pub relative_to: String,
    pub created_at: CreatedAt,
    pub registered_at: Location,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
/// A route registered against a `Blueprint` via `Blueprint::route`.
pub struct Route {
    pub coordinates: AnnotationCoordinates,
    /// The location where the component was registered against the `Blueprint`.
    pub registered_at: Location,
    /// The callable in charge of processing errors returned by this route, if any.
    pub error_handler: Option<ErrorHandler>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
/// A request handler registered against a `Blueprint` via `Blueprint::fallback` to
/// process requests that don't match any of the registered routes.
pub struct Fallback {
    pub coordinates: AnnotationCoordinates,
    /// The location where the component was registered against the `Blueprint`.
    pub registered_at: Location,
    /// The callable in charge of processing errors returned by this fallback, if any.
    pub error_handler: Option<ErrorHandler>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
/// An error observer registered against a `Blueprint` via `Blueprint::error_observer` to
/// intercept unhandled errors.
pub struct ErrorObserver {
    pub coordinates: AnnotationCoordinates,
    /// The location where the component was registered against the `Blueprint`.
    pub registered_at: Location,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
/// An error handler registered against a `Blueprint` via `Blueprint::error_handler`.
pub struct ErrorHandler {
    pub coordinates: AnnotationCoordinates,
    /// The location where the component was registered against the `Blueprint`.
    pub registered_at: Location,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
/// A type registered against a `Blueprint` via `Blueprint::prebuilt` to
/// be added as an input parameter to `ApplicationState::new`.
pub struct PrebuiltType {
    /// The coordinates of the annotated type.
    pub coordinates: AnnotationCoordinates,
    /// The strategy dictating when the prebuilt type can be cloned.
    pub cloning_policy: Option<CloningPolicy>,
    /// The location where this prebuilt type was registered in the application code.
    pub registered_at: Location,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
/// A type registered against a `Blueprint` via `Blueprint::config` to
/// become part of the overall configuration for the application.
pub struct ConfigType {
    /// The coordinates of the annotated type.
    pub coordinates: AnnotationCoordinates,
    /// The strategy dictating when the config type can be cloned.
    pub cloning_policy: Option<CloningPolicy>,
    /// Whether to use `Default::default` to generate default configuration
    /// values if the user hasn't specified any.
    pub default_if_missing: Option<bool>,
    /// Whether to include the config type as a field in the generated
    /// configuration struct even if it was never injected.
    pub include_if_unused: Option<bool>,
    /// The location where this configuration type was registered in the application code.
    pub registered_at: Location,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
/// A constructor registered against a `Blueprint` via `Blueprint::constructor`.
pub struct Constructor {
    /// The coordinates of the annotated constructor.
    pub coordinates: AnnotationCoordinates,
    /// The lifecycle of the constructed type.
    pub lifecycle: Option<Lifecycle>,
    /// The strategy dictating when the constructed type can be cloned.
    pub cloning_policy: Option<CloningPolicy>,
    /// The callable in charge of processing errors returned by this constructor, if any.
    pub error_handler: Option<ErrorHandler>,
    /// Lint settings for this constructor.
    pub lints: BTreeMap<Lint, LintSetting>,
    /// The location where this constructor was registered in the application code.
    pub registered_at: Location,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
/// A middleware registered against a `Blueprint` via `Blueprint::wrap`.
pub struct WrappingMiddleware {
    pub coordinates: AnnotationCoordinates,
    /// The location where the component was registered against the `Blueprint`.
    pub registered_at: Location,
    /// The callable in charge of processing errors returned by this middleware, if any.
    pub error_handler: Option<ErrorHandler>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
/// A middleware registered against a `Blueprint` via `Blueprint::post_process`.
pub struct PostProcessingMiddleware {
    pub coordinates: AnnotationCoordinates,
    /// The location where the component was registered against the `Blueprint`.
    pub registered_at: Location,
    /// The callable in charge of processing errors returned by this middleware, if any.
    pub error_handler: Option<ErrorHandler>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
/// A middleware registered against a `Blueprint` via `Blueprint::pre_process`.
pub struct PreProcessingMiddleware {
    pub coordinates: AnnotationCoordinates,
    /// The location where the component was registered against the `Blueprint`.
    pub registered_at: Location,
    /// The callable in charge of processing errors returned by this middleware, if any.
    pub error_handler: Option<ErrorHandler>,
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
pub enum CloningPolicy {
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
    /// Allow Pavex to [invoke a fallback error handler if no specific error handler is provided][1].
    ///
    /// [1]: https://pavex.dev/docs/guide/errors/error_handlers/#fallback-error-handler
    ErrorFallback,
}

#[derive(
    Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum LintSetting {
    Allow,
    Warn,
    Deny,
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

#[derive(Clone, Debug, Eq, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
/// A set of coordinates to identify a precise spot in a source file.
///
/// # Implementation Notes
///
/// `Location` is an owned version of [`std::panic::Location`].
/// You can build a `Location` instance starting from a [`std::panic::Location`]:
///
/// ```rust
/// use pavex_bp_schema::Location;
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

impl Location {
    #[track_caller]
    pub fn caller() -> Self {
        std::panic::Location::caller().into()
    }
}

#[derive(Debug, Hash, Eq, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
/// The method used to create (and set the properties) for this component.
pub enum CreatedBy {
    /// The component was created via a macro annotation (e.g. `#[pavex::wrap]`)
    /// on top of the target item (e.g. a function or a method).
    Attribute { name: String },
    /// The component was provided by the framework.
    ///
    /// For example, the default fallback handler if the user didn't specify one.
    Framework,
}

impl CreatedBy {
    /// Convert the name of the macro used to perform the registration into an instance of [`CreatedBy`].
    pub fn macro_name(value: &str) -> Self {
        match value {
            "pre_process" | "post_process" | "wrap" | "request_scoped" | "transient"
            | "singleton" | "config" | "error_handler" | "error_observer" | "fallback"
            | "route" => CreatedBy::Attribute { name: value.into() },
            _ => panic!(
                "Pavex doesn't recognize `{value}` as one of its macros to register components"
            ),
        }
    }
}

#[derive(Debug, Hash, Eq, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct AnnotationCoordinates {
    /// An opaque string that uniquely identifies this component within the package
    /// where it was defined.
    pub id: String,
    /// Metadata required to pinpoint where the annotated component lives.
    pub created_at: CreatedAt,
    /// The name of the macro used to annotate the component.
    pub macro_name: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
/// Information on the crate/module where the component was created.
///
/// This location matches, for example, where the `from!` or the `f!` macro were invoked.
/// For annotated items (e.g. via `#[pavex::config]`), this refers to the location of the annotation.
///
/// It may be different from the location where the component was registered
/// with the blueprint—i.e. where a `Blueprint` method was invoked.
pub struct CreatedAt {
    /// The name of the crate that created the component, as it appears in the `package.name` field
    /// of its `Cargo.toml`.
    /// Obtained via [`CARGO_PKG_NAME`](https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-crates).
    ///
    /// In particular, the name has *not* been normalised—e.g. hyphens are not replaced with underscores.
    ///
    /// This information is needed to resolve the import path unambiguously.
    ///
    /// E.g. `my_crate::module_1::type_2`—which crate is `my_crate`?
    /// This is not obvious due to the possibility of [renaming dependencies in `Cargo.toml`](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html?highlight=rename,depende#renaming-dependencies-in-cargotoml):
    ///
    /// ```toml
    /// [package]
    /// name = "mypackage"
    /// version = "0.0.1"
    ///
    /// [dependencies]
    /// my_crate = { version = "0.1", registry = "custom", package = "their_crate" }
    /// ```
    pub package_name: String,
    /// The version of the crate that created the component, as it appears in the `package.version` field
    /// of its `Cargo.toml`.
    ///
    /// Obtained via [`CARGO_PKG_VERSION`](https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-crates).
    pub package_version: String,
}
