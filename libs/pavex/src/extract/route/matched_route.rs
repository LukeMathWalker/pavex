use std::fmt::Formatter;

#[derive(Debug, Clone, Copy)]
/// The route template that matched for the incoming request.
///
/// # Example
///
/// If you configure your [`Blueprint`] like this:
///
/// ```rust
/// use pavex::{f, blueprint::{Blueprint, router::GET}};
/// # use pavex::{request::RequestHead, response::Response};
/// # fn get_home(request: RequestHead) -> Response { todo!() }
/// # fn main() {
/// # let mut bp = Blueprint::new();
///
/// bp.route(GET, "/home/:home_id", f!(crate::get_home));
/// # }
/// ```
///
/// Then [`MatchedRouteTemplate`] will be set to `/home/:home_id` for a `GET /home/123` request.
///
/// # Framework primitive
///
/// `MatchedRouteTemplate` is a framework primitive—you don't need to register any constructor
/// with [`Blueprint`] to use it in your application.
///
/// # Use cases
///
/// The primary use case for [`MatchedRouteTemplate`] is telemetry—logging, metrics, etc.  
/// It lets you strip away the dynamic parts of the request path, thus reducing the cardinality of
/// your metrics and making it easier to aggregate them.
///
/// [`Blueprint`]: crate::blueprint::Blueprint
#[doc(alias("MatchedPath"))]
pub struct MatchedRouteTemplate(&'static str);

impl MatchedRouteTemplate {
    /// Create a new matched route from a route template.
    ///
    /// # Example
    ///
    /// ```rust
    /// use pavex::extract::route::MatchedRouteTemplate;
    ///
    /// let matched_route = MatchedRouteTemplate::new("/home/:home_id");
    /// ```
    pub fn new(route: &'static str) -> Self {
        Self(route)
    }

    /// Get a reference to the underlying route template.
    pub fn inner(self) -> &'static str {
        self.0
    }
}

impl std::fmt::Display for MatchedRouteTemplate {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
