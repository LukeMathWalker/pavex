//! Internal types used by [`Blueprint`]s to keep track of what you registered.
//! 
//! This module is not meant to be used directly by users of the framework. It is only meant to be
//! used by Pavex's CLI.
use super::constructor::{CloningStrategy, Lifecycle};
use super::reflection::{Location, RawCallableIdentifiers};
use super::router::MethodGuard;
use super::Blueprint;

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
/// A constructor registered against a [`Blueprint`] via [`Blueprint::constructor`].
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
/// A "callable" registered against a [`Blueprint`]—either a free function or a method,
/// used as a request handler, error handler or constructor.
pub struct RegisteredCallable {
    /// Metadata that uniquely identifies the callable.
    pub callable: RawCallableIdentifiers,
    /// The location where the callable was registered against the [`Blueprint`].
    pub location: Location,
}

#[derive(serde::Serialize, serde::Deserialize)]
/// A [`Blueprint`] that has been nested inside another [`Blueprint`] via [`Blueprint::nest`] or
/// [`Blueprint::nest_at`].
pub struct NestedBlueprint {
    /// The nested [`Blueprint`].
    pub blueprint: Blueprint,
    /// The path prefix that will prepended to all routes registered against the nested
    /// [`Blueprint`].  
    /// If `None`, the routes coming from the nested [`Blueprint`] will be registered as-they-are.
    pub path_prefix: Option<String>,
    /// The location where the [`Blueprint`] was nested under its parent [`Blueprint`].
    pub nesting_location: Location,
}
