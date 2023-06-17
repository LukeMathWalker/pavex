use std::collections::HashSet;
use std::str::FromStr;

use serde::ser::SerializeSeq;
use serde::{Deserializer, Serializer};

use http::Method;

/// Match incoming requests based on their HTTP method.
///
/// Used by [`Blueprint::route`] to specify which HTTP methods the route should match.
///
/// If you want to match **any** HTTP method, use [`ANY`].  
/// If you want to match a single HTTP method, use the dedicated constants in this
/// module ([`GET`], [`POST`], [`PATCH`], [`DELETE`], etc.).  
/// If you want to match a list of HTTP methods, use [`MethodGuard::new`].  
///
/// [`Blueprint::route`]: crate::blueprint::Blueprint::route
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MethodGuard {
    // TODO: it should not be public, even if it's hidden
    #[doc(hidden)]
    pub allowed_methods: AllowedMethods,
}

impl MethodGuard {
    /// Build a new [`MethodGuard`] that matches the specified list of HTTP methods.
    ///
    /// ```rust
    /// use pavex::blueprint::router::MethodGuard;
    /// use pavex::http::Method;
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
// TODO: it should not be public, even if it's hidden
#[doc(hidden)]
pub enum AllowedMethods {
    All,
    Single(Method),
    Multiple(HashSet<Method>),
}

impl serde::Serialize for AllowedMethods {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(None)?;
        match self {
            AllowedMethods::All => seq.serialize_element("*")?,
            AllowedMethods::Single(method) => seq.serialize_element(method.as_str())?,
            AllowedMethods::Multiple(methods) => {
                for method in methods {
                    seq.serialize_element(method.as_str())?;
                }
            }
        }
        seq.end()
    }
}

impl<'de> serde::Deserialize<'de> for AllowedMethods {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let methods: Vec<String> = serde::de::Deserialize::deserialize(deserializer)?;
        if methods.is_empty() {
            return Err(serde::de::Error::custom("expected at least one method"));
        }
        if methods.len() == 1 {
            if methods[0] == "*" {
                return Ok(AllowedMethods::All);
            }
            return match Method::from_str(&methods[0]) {
                Ok(method) => Ok(AllowedMethods::Single(method)),
                Err(e) => Err(serde::de::Error::custom(format!("invalid method: {}", e))),
            };
        }
        let mut set = HashSet::new();
        for method in methods {
            let method = match Method::from_str(&method) {
                Ok(method) => method,
                Err(e) => return Err(serde::de::Error::custom(format!("invalid method: {}", e))),
            };
            set.insert(method);
        }
        Ok(AllowedMethods::Multiple(set))
    }
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
