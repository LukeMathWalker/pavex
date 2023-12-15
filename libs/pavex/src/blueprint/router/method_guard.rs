use std::collections::BTreeSet;

use http::Method;

use crate::blueprint::router::method_guard::inner::method_to_bitset;
use crate::router::allowed_methods::MethodAllowList;
use crate::router::AllowedMethods;

/// Used by [`Blueprint::route`] to specify which HTTP methods the route should match.
///
/// If you want to match **any** HTTP method, use [`ANY`].  
/// If you want to match a single HTTP method, use the dedicated constants in this
/// module ([`GET`], [`POST`], [`PATCH`], [`DELETE`], etc.).  
/// If you want to match a list of HTTP methods, use either [`MethodGuard::or`] or
/// [`MethodGuard::from_iter`].
///
/// [`Blueprint::route`]: crate::blueprint::Blueprint::route
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MethodGuard {
    inner: inner::MethodGuard,
}

impl MethodGuard {
    /// Build a new [`MethodGuard`] that matches incoming requests using the given HTTP methods.
    ///
    /// ```rust
    /// use pavex::blueprint::router::{MethodGuard, GET, POST};
    /// use pavex::http::Method;
    ///
    /// // Using an array of methods known at compile-time..
    /// let guard = MethodGuard::from_iter([Method::GET, Method::POST]);
    /// // ..or a dynamic vector, built at runtime.
    /// let guard = MethodGuard::from_iter(vec![Method::GET, Method::PUT]);
    ///
    /// // As an alternative, you can use the `or` method to combine two or more `MethodGuard`s.
    /// // It's usually shorter and more readable.
    /// let guard = GET.or(POST);
    /// ```
    ///
    /// If you want to match **any** HTTP method, use [`ANY`].  
    /// If you want to match a single HTTP method, use the dedicated constants in this
    /// module ([`GET`], [`POST`], [`PATCH`], [`DELETE`], etc.).
    pub fn from_iter(allowed_methods: impl IntoIterator<Item = Method>) -> Self {
        let mut bitset = 0;
        let mut extensions = BTreeSet::new();
        for method in allowed_methods {
            let method = inner::Method::from(method);
            if let Some(bit) = method_to_bitset(&method) {
                bitset |= bit;
            } else {
                extensions.insert(method);
            }
        }
        MethodGuard {
            inner: inner::MethodGuard::Some(inner::SomeMethodGuard { bitset, extensions }),
        }
    }

    /// Combine this [`MethodGuard`] with another one, returning a new [`MethodGuard`].
    ///
    /// The returned [`MethodGuard`] will match requests that match either of the two
    /// [`MethodGuard`]s.
    ///
    /// ```rust
    /// use pavex::blueprint::router::{GET, POST};
    ///
    /// // Match requests that use either the `GET` or the `POST` HTTP method.
    /// let guard = GET.or(POST);
    /// ```
    pub fn or(self, other: MethodGuard) -> Self {
        MethodGuard {
            inner: self.inner.or(other.inner),
        }
    }

    /// Returns `true` if the given HTTP method is allowed by this [`MethodGuard`].
    pub fn allows(&self, method: Method) -> bool {
        self.allows_(&inner::Method::from(method))
    }

    fn allows_(&self, method: &inner::Method) -> bool {
        match &self.inner {
            inner::MethodGuard::Any => true,
            inner::MethodGuard::Some(inner::SomeMethodGuard { bitset, extensions }) => {
                if let Some(bit) = method_to_bitset(method) {
                    *bitset & bit != 0
                } else {
                    extensions.contains(method)
                }
            }
        }
    }

    /// Return the methods allowed by this [`MethodGuard`].
    pub fn allowed_methods(&self) -> AllowedMethods {
        match &self.inner {
            inner::MethodGuard::Any => AllowedMethods::All,
            inner::MethodGuard::Some(inner::SomeMethodGuard {
                bitset: _,
                extensions,
            }) => {
                let methods = extensions
                    .iter()
                    .cloned()
                    .chain(
                        [
                            inner::Method::GET,
                            inner::Method::POST,
                            inner::Method::PATCH,
                            inner::Method::OPTIONS,
                            inner::Method::PUT,
                            inner::Method::DELETE,
                            inner::Method::TRACE,
                            inner::Method::HEAD,
                            inner::Method::CONNECT,
                        ]
                        .into_iter()
                        .filter(|method| self.allows_(method)),
                    )
                    .map(Method::from);
                AllowedMethods::Some(MethodAllowList::from_iter(methods))
            }
        }
    }
}

impl From<Method> for MethodGuard {
    fn from(val: Method) -> Self {
        let method = inner::Method::from(val);
        let inner = if let Some(bit) = method_to_bitset(&method) {
            inner::MethodGuard::Some(inner::SomeMethodGuard {
                bitset: bit,
                extensions: BTreeSet::new(),
            })
        } else {
            let mut extensions = BTreeSet::new();
            extensions.insert(method);
            inner::MethodGuard::Some(inner::SomeMethodGuard {
                bitset: 0,
                extensions,
            })
        };
        MethodGuard { inner }
    }
}

/// A [`MethodGuard`] that matches incoming requests with a well-known HTTP method:
/// `CONNECT`, `DELETE`, `GET`, `HEAD`, `PATCH`, `POST`, `PUT`, `OPTIONS`, `TRACE`.
///
/// If you want to allow custom HTTP methods in addition to well-known ones,
/// use [`ANY_WITH_EXTENSIONS`].
pub const ANY: MethodGuard = MethodGuard { inner: inner::ANY };

/// A [`MethodGuard`] that matches all incoming requests, no matter their HTTP method,
/// even if it's a custom one.
///
/// If you only want to allow well-known HTTP methods, use [`ANY`].
pub const ANY_WITH_EXTENSIONS: MethodGuard = MethodGuard {
    inner: inner::ANY_WITH_EXTENSIONS,
};

/// A [`MethodGuard`] that matches incoming requests using the `GET` HTTP method.
pub const GET: MethodGuard = MethodGuard { inner: inner::GET };

/// A [`MethodGuard`] that matches incoming requests using the `POST` HTTP method.
pub const POST: MethodGuard = MethodGuard { inner: inner::POST };

/// A [`MethodGuard`] that matches incoming requests using the `PATCH` HTTP method.
pub const PATCH: MethodGuard = MethodGuard {
    inner: inner::PATCH,
};

/// A [`MethodGuard`] that matches incoming requests using the `OPTIONS` HTTP method.
pub const OPTIONS: MethodGuard = MethodGuard {
    inner: inner::OPTIONS,
};

/// A [`MethodGuard`] that matches incoming requests using the `PUT` HTTP method.
pub const PUT: MethodGuard = MethodGuard { inner: inner::PUT };

/// A [`MethodGuard`] that matches incoming requests using the `DELETE` HTTP method.
pub const DELETE: MethodGuard = MethodGuard {
    inner: inner::DELETE,
};

/// A [`MethodGuard`] that matches incoming requests using the `TRACE` HTTP method.
pub const TRACE: MethodGuard = MethodGuard {
    inner: inner::TRACE,
};

/// A [`MethodGuard`] that matches incoming requests using the `HEAD` HTTP method.
pub const HEAD: MethodGuard = MethodGuard { inner: inner::HEAD };

/// A [`MethodGuard`] that matches incoming requests using the `CONNECT` HTTP method.
pub const CONNECT: MethodGuard = MethodGuard {
    inner: inner::CONNECT,
};

mod inner {
    #![allow(clippy::upper_case_acronyms)]

    use std::collections::BTreeSet;
    use std::str::FromStr;

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub(super) enum MethodGuard {
        Any,
        Some(SomeMethodGuard),
    }

    /// In order to have a const constructor for `MethodGuard`, we need to use a collection
    /// for extension methods that can be created in a const context.
    ///
    /// There's only two options at the moment: `BTreeSet` and `Vec`.  
    /// `Vec` wouldn't give us deduplication, but we can't use `BTreeSet` with `http::Method` because
    /// it doesn't implement `Ord`.
    ///
    /// To work around this, we use a custom `Method` enum that implements `Ord`, `PartialOrd`,
    /// `Serialize` and `Deserialize`.
    #[derive(
        Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq, PartialOrd, Ord,
    )]
    pub(super) enum Method {
        GET,
        POST,
        PATCH,
        OPTIONS,
        PUT,
        DELETE,
        TRACE,
        HEAD,
        CONNECT,
        Custom(String),
    }

    impl From<http::Method> for Method {
        fn from(method: http::Method) -> Self {
            match method {
                http::Method::GET => Method::GET,
                http::Method::POST => Method::POST,
                http::Method::PATCH => Method::PATCH,
                http::Method::OPTIONS => Method::OPTIONS,
                http::Method::PUT => Method::PUT,
                http::Method::DELETE => Method::DELETE,
                http::Method::TRACE => Method::TRACE,
                http::Method::HEAD => Method::HEAD,
                http::Method::CONNECT => Method::CONNECT,
                m => Method::Custom(m.as_str().to_string()),
            }
        }
    }

    impl From<Method> for http::Method {
        fn from(value: Method) -> Self {
            match value {
                Method::GET => http::Method::GET,
                Method::POST => http::Method::POST,
                Method::PATCH => http::Method::PATCH,
                Method::OPTIONS => http::Method::OPTIONS,
                Method::PUT => http::Method::PUT,
                Method::DELETE => http::Method::DELETE,
                Method::TRACE => http::Method::TRACE,
                Method::HEAD => http::Method::HEAD,
                Method::CONNECT => http::Method::CONNECT,
                Method::Custom(c) => http::Method::from_str(c.as_str()).unwrap(),
            }
        }
    }

    impl MethodGuard {
        pub(super) fn or(self, other: MethodGuard) -> Self {
            match (self, other) {
                (MethodGuard::Any, _) | (_, MethodGuard::Any) => MethodGuard::Any,
                (MethodGuard::Some(this), MethodGuard::Some(other)) => {
                    MethodGuard::Some(this.or(other))
                }
            }
        }

        const fn from_bits(bitset: u16) -> Self {
            MethodGuard::Some(SomeMethodGuard {
                bitset,
                extensions: BTreeSet::new(),
            })
        }
    }

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub(super) struct SomeMethodGuard {
        /// A bitset to track which of the 9 well-known HTTP methods are allowed.
        ///
        /// # Why so complicated?
        ///
        /// We don't use a bitset because we want to be "low overhead": `MethodGuard` is only
        /// used when assembling a `Blueprint`, it doesn't play any role at runtime.  
        /// We use a bitset, rather than a `BTreeSet`, because we want to be able to expose
        /// a constant for each well-known HTTP method, and we can't use data structures that
        /// allocate memory at runtime (such as `BTreeSet`) in a `const` context.
        pub(super) bitset: u16,
        pub(super) extensions: BTreeSet<Method>,
    }

    // `Method` doesn't implement `Serialize` and `Deserialize, therefore we need to implement
    // a custom serializer and deserializer for `SomeMethodGuard`.

    impl SomeMethodGuard {
        pub(super) fn or(mut self, other: SomeMethodGuard) -> Self {
            self.bitset |= other.bitset;
            self.extensions.extend(other.extensions);
            self
        }
    }

    pub(super) const fn method_to_bitset(method: &Method) -> Option<u16> {
        match method {
            &Method::GET
            | &Method::POST
            | &Method::PATCH
            | &Method::OPTIONS
            | &Method::PUT
            | &Method::DELETE
            | &Method::TRACE
            | &Method::HEAD
            | &Method::CONNECT => Some(_method_to_bitset(method)),
            _ => None,
        }
    }

    // We can't call `unwrap` in a `const` context because const panics do not support formatted
    // panic messages.
    // This is why we use this function instead of `method_to_bitset` directly in the `const`
    // declarations below.
    const fn _method_to_bitset(method: &Method) -> u16 {
        match *method {
            Method::GET => 0b0000_0001_0000_0000,
            Method::POST => 0b0000_0000_1000_0000,
            Method::PATCH => 0b0000_0000_0100_0000,
            Method::OPTIONS => 0b0000_0000_0010_0000,
            Method::PUT => 0b0000_0000_0001_0000,
            Method::DELETE => 0b0000_0000_0000_1000,
            Method::TRACE => 0b0000_0000_0000_0100,
            Method::HEAD => 0b0000_0000_0000_0010,
            Method::CONNECT => 0b0000_0000_0000_0001,
            Method::Custom(_) => panic!(),
        }
    }

    pub(super) const GET: MethodGuard = MethodGuard::from_bits(_method_to_bitset(&Method::GET));
    pub(super) const POST: MethodGuard = MethodGuard::from_bits(_method_to_bitset(&Method::POST));
    pub(super) const PATCH: MethodGuard = MethodGuard::from_bits(_method_to_bitset(&Method::PATCH));
    pub(super) const OPTIONS: MethodGuard =
        MethodGuard::from_bits(_method_to_bitset(&Method::OPTIONS));
    pub(super) const PUT: MethodGuard = MethodGuard::from_bits(_method_to_bitset(&Method::PUT));
    pub(super) const DELETE: MethodGuard =
        MethodGuard::from_bits(_method_to_bitset(&Method::DELETE));
    pub(super) const TRACE: MethodGuard = MethodGuard::from_bits(_method_to_bitset(&Method::TRACE));
    pub(super) const HEAD: MethodGuard = MethodGuard::from_bits(_method_to_bitset(&Method::HEAD));
    pub(super) const CONNECT: MethodGuard =
        MethodGuard::from_bits(_method_to_bitset(&Method::CONNECT));
    pub(super) const ANY: MethodGuard = MethodGuard::from_bits(0b0000_0001_1111_1111);
    pub(super) const ANY_WITH_EXTENSIONS: MethodGuard = MethodGuard::Any;
}
