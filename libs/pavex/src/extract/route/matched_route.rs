use std::fmt::Formatter;

#[derive(Debug, Clone, Copy)]
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
