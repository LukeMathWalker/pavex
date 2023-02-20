use pavex_runtime::http::Method;

/// Match incoming requests based on their HTTP method.
///
/// Used by [`Blueprint::route`] to specify which HTTP methods the route should match.
///
/// If you want to match **any** HTTP method, use [`ANY`].  
/// If you want to match a single HTTP method, use the dedicated constants in this
/// module ([`GET`], [`POST`], [`PATCH`], [`DELETE`], etc.).  
/// If you want to match a list of HTTP methods, use [`MethodGuard::new`].  
///
/// [`Blueprint::route`]: crate::Blueprint::route
#[derive(Debug, Clone)]
pub struct MethodGuard {
    allowed_methods: AllowedMethods,
}

impl MethodGuard {
    /// Build a new [`MethodGuard`] that matches the specified list of HTTP methods.
    ///
    /// ```rust
    /// use pavex_builder::router::MethodGuard;
    /// use pavex_runtime::http::Method;
    ///
    /// // Using an array of methods known at compile-time..
    /// let guard = MethodGuard::new([Method::GET, Method::POST]);
    /// // ..or a dynamic vector, built at runtime.
    /// let guard = MethodGuard::new(vec![Method::GET, Method::PUT]);
    /// ```
    ///
    /// If you want to match **any** HTTP method, use [`ANY`].  
    /// If you want to match a single HTTP method, use the dedicated constants in this
    /// module ([`GET`], [`POST`], [`PATCH`], [`DELETE`], etc.).
    pub fn new(allowed_methods: impl IntoIterator<Item = Method>) -> Self {
        let allowed_methods = AllowedMethods::Multiple(allowed_methods.into_iter().collect());
        Self { allowed_methods }
    }
}

#[derive(Debug, Clone)]
enum AllowedMethods {
    All,
    Single(Method),
    Multiple(Vec<Method>),
}

/// A [`MethodGuard`] that matches all incoming requests, regardless of their HTTP method.
pub const ANY: MethodGuard = MethodGuard {
    allowed_methods: AllowedMethods::All,
};

/// A [`MethodGuard`] that matches incoming requests using the `GET` HTTP method.
pub const GET: MethodGuard = MethodGuard {
    allowed_methods: AllowedMethods::Single(Method::GET),
};

/// A [`MethodGuard`] that matches incoming requests using the `POST` HTTP method.
pub const POST: MethodGuard = MethodGuard {
    allowed_methods: AllowedMethods::Single(Method::POST),
};

/// A [`MethodGuard`] that matches incoming requests using the `PATCH` HTTP method.
pub const PATCH: MethodGuard = MethodGuard {
    allowed_methods: AllowedMethods::Single(Method::PATCH),
};

/// A [`MethodGuard`] that matches incoming requests using the `OPTIONS` HTTP method.
pub const OPTIONS: MethodGuard = MethodGuard {
    allowed_methods: AllowedMethods::Single(Method::OPTIONS),
};

/// A [`MethodGuard`] that matches incoming requests using the `PUT` HTTP method.
pub const PUT: MethodGuard = MethodGuard {
    allowed_methods: AllowedMethods::Single(Method::PUT),
};

/// A [`MethodGuard`] that matches incoming requests using the `DELETE` HTTP method.
pub const DELETE: MethodGuard = MethodGuard {
    allowed_methods: AllowedMethods::Single(Method::DELETE),
};

/// A [`MethodGuard`] that matches incoming requests using the `TRACE` HTTP method.
pub const TRACE: MethodGuard = MethodGuard {
    allowed_methods: AllowedMethods::Single(Method::TRACE),
};

/// A [`MethodGuard`] that matches incoming requests using the `HEAD` HTTP method.
pub const HEAD: MethodGuard = MethodGuard {
    allowed_methods: AllowedMethods::Single(Method::HEAD),
};

/// A [`MethodGuard`] that matches incoming requests using the `CONNECT` HTTP method.
pub const CONNECT: MethodGuard = MethodGuard {
    allowed_methods: AllowedMethods::Single(Method::CONNECT),
};
