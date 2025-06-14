use std::fmt::Formatter;

#[derive(Debug, Clone, Copy)]
/// The route template that matched for the incoming request.
///
/// # Example
///
/// If you configure your [`Blueprint`] like this:
///
/// ```rust
/// use pavex::{get, response::Response};
/// #[get(path = "/home/{home_id}")]
/// pub fn get_home(/* [...] */) -> Response {
///     // [...]
///     # Response::ok()
/// }
/// ```
///
/// Then [`MatchedPathPattern`] will be set to `/home/{home_id}` for a `GET /home/123` request.
///
/// # Framework primitive
///
/// `MatchedPathPattern` is a framework primitive—you don't need to register any constructor
/// with [`Blueprint`] to use it in your application.
///
/// # Use cases
///
/// The primary use case for [`MatchedPathPattern`] is telemetry—logging, metrics, etc.
/// It lets you strip away the dynamic parts of the request path, thus reducing the cardinality of
/// your metrics and making it easier to aggregate them.
///
/// [`Blueprint`]: crate::blueprint::Blueprint
#[doc(alias("MatchedPath"))]
#[doc(alias("MatchedPathTemplate"))]
#[doc(alias("PathPattern"))]
#[doc(alias("PathTemplate"))]
#[doc(alias("MatchedRoute"))]
#[doc(alias("MatchedRouteTemplate"))]
pub struct MatchedPathPattern(&'static str);

impl MatchedPathPattern {
    /// Create a new matched route from a route template.
    ///
    /// # Example
    ///
    /// ```rust
    /// use pavex::request::path::MatchedPathPattern;
    ///
    /// let matched_route = MatchedPathPattern::new("/home/:home_id");
    /// ```
    pub fn new(route: &'static str) -> Self {
        Self(route)
    }

    /// Get a reference to the underlying route template.
    pub fn inner(self) -> &'static str {
        self.0
    }
}

impl std::fmt::Display for MatchedPathPattern {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
