use crate::http::Method;

/// The list of HTTP methods that are allowed for a given path.
///
/// # Example
///
/// ```rust
/// use pavex::extract::route::AllowedMethods;
/// use pavex::response::Response;
/// use pavex::http::header::{ALLOW, HeaderValue};
/// use itertools::Itertools;
///
/// /// A fallback handler that returns a `404 Not Found` if the request path
/// /// doesn't match any of the registered routes, or a `405 Method Not Allowed`
/// /// if the request path matches a registered route but there is no handler
/// /// for its HTTP method.
/// pub async fn fallback(allowed_methods: AllowedMethods) -> Response {
///     if allowed_methods.len() == 0 {
///         Response::not_found().box_body()
///     } else {
///         let allow_header = allowed_methods
///             .into_iter()
///             .join(",");
///         let allow_header = HeaderValue::from_str(&allow_header).unwrap();
///         Response::method_not_allowed()
///             .insert_header(ALLOW, allow_header)
///             .box_body()
///     }
/// }
///
/// ```
///
/// # Framework primitive
///
/// `AllowedMethods` is a framework primitive—you don't need to register any constructor
/// with [`Blueprint`] to use it in your application.
///
/// # Use cases
///
/// [`AllowedMethods`] comes into the play when implementing [fallback handlers]: it is necessary
/// to set the `Allow` header to the correct value when returning a `405 Method Not Allowed`
/// response after a routing failure.
///
/// [`Blueprint`]: crate::blueprint::Blueprint
/// [fallback handlers]: crate::blueprint::Blueprint::fallback
#[derive(Debug, Clone)]
pub struct AllowedMethods(Vec<Method>);

impl AllowedMethods {
    /// Create a new `AllowedMethods` instance.
    pub fn new(methods: Vec<Method>) -> Self {
        Self(methods)
    }

    /// Iterate over the allowed methods, returned as a reference.
    pub fn iter(&self) -> impl Iterator<Item = &Method> {
        self.0.iter()
    }

    /// Consume `self` and return an iterator over the allowed methods.
    pub fn into_iter(self) -> impl Iterator<Item = Method> {
        self.0.into_iter()
    }

    /// Get the number of allowed methods.
    pub fn len(&self) -> usize {
        self.0.len()
    }
}
