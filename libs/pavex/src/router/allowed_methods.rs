use http::HeaderValue;
use smallvec::SmallVec;

use crate::http::Method;

/// The list of HTTP methods that are allowed for a given path.
///
/// # Example
///
/// ```rust
/// use pavex::router::AllowedMethods;
/// use pavex::response::Response;
/// use pavex::http::header::{ALLOW, HeaderValue};
/// use itertools::Itertools;
///
/// /// A fallback handler that returns a `404 Not Found` if the request path
/// /// doesn't match any of the registered routes, or a `405 Method Not Allowed`
/// /// if the request path matches a registered route but there is no handler
/// /// for its HTTP method.
/// pub async fn fallback(allowed_methods: AllowedMethods) -> Response {
///     if let Some(header_value) = allowed_methods.allow_header_value() {
///         Response::method_not_allowed()
///             .insert_header(ALLOW, header_value)
///             .box_body()
///     } else {
///         Response::not_found().box_body()
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
pub enum AllowedMethods {
    Some(MethodAllowList),
    All(AllMethods),
}

impl AllowedMethods {
    /// The value that should be set for the `Allow` header
    /// in a `405 Method Not Allowed` response for this route path.
    ///
    /// It returns `None` if all methods are allowed, including custom ones.  
    /// It returns the comma-separated list of accepted HTTP methods otherwise.
    pub fn allow_header_value(&self) -> Option<HeaderValue> {
        match self {
            AllowedMethods::Some(m) => m.allow_header_value(),
            AllowedMethods::All(m) => m.allow_header_value(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MethodAllowList {
    // We use 5 as our inlining limit because that's going to fit
    // all methods in the most common case
    // (i.e. `GET`/`POST`/`PUT`/`DELETE`/`PATCH` on a certain route path).
    methods: SmallVec<[Method; 5]>,
}

#[derive(Debug, Clone)]
pub struct AllMethods {
    include_extensions: bool,
}

impl AllMethods {
    /// The value that should be set for the `Allow` header
    /// in a `405 Method Not Allowed` response for this route path.
    ///
    /// It returns `None` if custom methods are allowed.
    /// It returns the comma-separated list of all well-known HTTP methods otherwise.
    pub const fn allow_header_value(&self) -> Option<HeaderValue> {
        if self.include_extensions {
            None
        } else {
            Some(HeaderValue::from_static(
                "CONNECT,DELETE,GET,HEAD,PATCH,POST,PUT,OPTIONS,TRACE",
            ))
        }
    }

    /// Returns `true` if custom HTTP methods are allowed, `false` otherwise.
    pub const fn allows_extensions(&self) -> bool {
        self.include_extensions
    }

    /// All HTTP methods are allowed, including custom ones.
    ///
    /// Use [`AllMethods::with_extensions`] if you don't want to allow custom methods.
    pub const fn with_extensions() -> Self {
        Self {
            include_extensions: true,
        }
    }

    /// All well-known HTTP methods are allowed.  
    /// Custom ones are rejected.
    ///
    /// Use [`AllMethods::with_extensions`] if you want to allow custom methods.
    pub const fn without_extensions() -> Self {
        Self {
            include_extensions: false,
        }
    }
}

impl MethodAllowList {
    /// Create a new instance of [`MethodAllowList`] from an iterator
    /// that yields [`Method`]s.
    pub fn from_iter(iter: impl IntoIterator<Item = Method>) -> Self {
        Self {
            methods: SmallVec::from_iter(iter),
        }
    }

    /// Iterate over the allowed methods, returned as a reference.
    pub fn iter(&self) -> impl Iterator<Item = &Method> {
        self.methods.iter()
    }

    /// Consume `self` and return an iterator over the allowed methods.
    pub fn into_iter(self) -> impl Iterator<Item = Method> {
        self.methods.into_iter()
    }

    /// Get the number of allowed methods.
    pub fn len(&self) -> usize {
        self.methods.len()
    }

    /// Check if there are no allowed methods.
    pub fn is_empty(&self) -> bool {
        self.methods.is_empty()
    }

    /// The value that should be set for the `Allow` header
    /// in a `405 Method Not Allowed` response for this route path.
    ///
    /// It returns `None` if there are no allowed methods.
    /// It returns the comma-separated list of allowed methods otherwise.
    pub fn allow_header_value(&self) -> Option<HeaderValue> {
        if self.methods.is_empty() {
            None
        } else {
            let allow_header = join(&mut self.methods.iter().map(|method| method.as_str()), ",");
            Some(
                HeaderValue::from_str(&allow_header)
                    .expect("Failed to assemble `Allow` header value"),
            )
        }
    }
}

// Inlined from `itertools to avoid adding a dependency.
fn join<'a, I>(iter: &mut I, separator: &str) -> String
where
    I: Iterator<Item = &'a str>,
{
    use std::fmt::Write;

    match iter.next() {
        None => String::new(),
        Some(first_elt) => {
            let mut result = String::with_capacity(separator.len() * iter.size_hint().0);
            write!(&mut result, "{}", first_elt).unwrap();
            iter.for_each(|element| {
                result.push_str(separator);
                write!(&mut result, "{}", element).unwrap();
            });
            result
        }
    }
}
