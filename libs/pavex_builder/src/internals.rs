use crate::reflection::{Location, RawCallableIdentifiers};
use crate::router::MethodGuard;
use crate::Blueprint;

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
    pub location: Location,
}
