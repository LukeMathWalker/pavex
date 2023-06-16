use bytes::Bytes;
use http::header::CONTENT_TYPE;
use http_body::Empty;

use super::body::raw::{boxed, BoxBody};
use super::body::TypedBody;
use crate::http::StatusCode;
use crate::http::{HeaderMap, Version};

/// Represents an HTTP response.
///
/// ```rust
/// use pavex::response::Response;
/// use pavex::http::{HeaderValue, header::SERVER};
///
/// // Create a new response with:
/// // - status code `OK`
/// // - HTTP version `HTTP/1.1`
/// // - the `Server` header set to `Pavex`
/// // - the `Content-Type` header set to `text/plain; charset=utf-8`
/// // - the body set to `Hello, world!`
/// let response = Response::ok()
///     .insert_header(SERVER, HeaderValue::from_static("Pavex"))
///     .set_typed_body("Hello, world!");
/// ```
///
/// The response is composed of a head ([`ResponseHead`]) and an optional body.  
///
/// Check out [`Response::new`] for details on how to build a new [`Response`].  
/// You might also want to check out the following methods to further customize
/// your response:
///
/// - [`set_status`](Response::set_status) to change the status code.
/// - [`set_version`](Response::set_version) to change the HTTP version.
/// - [`append_header`](Response::append_header) to append a value to a header.
/// - [`insert_header`](Response::insert_header) to upsert a header value.
/// - [`set_typed_body`](Response::set_typed_body) to set the body and automatically set the `Content-Type` header.
///
/// There are other methods available on [`Response`] that you might find useful, but the
/// ones listed above are the most commonly used and should be enough to get you started.
pub struct Response<Body = BoxBody> {
    inner: http::Response<Body>,
}

#[non_exhaustive]
#[derive(Debug)]
/// All the information that is transmitted as part of an HTTP [`Response`] ahead of the body.
///
/// It includes the status code, the HTTP version, and the headers.
pub struct ResponseHead {
    status: StatusCode,
    version: Version,
    headers: HeaderMap,
}

impl Response<Empty<Bytes>> {
    /// Build a new [`Response`] with the given status code.  
    /// The HTTP version is set to HTTP 1.1, there are no headers and
    /// the body is empty.
    ///
    /// # Example
    ///
    /// ```rust
    /// use pavex::http::StatusCode;
    /// use pavex::response::Response;
    ///     
    /// let response = Response::new(StatusCode::OK);
    /// ```
    ///
    /// # Alternatives
    ///
    /// Pavex's provides a set of shorthands for building a new [`Response`] using
    /// well-known status code. For example, the following code is equivalent to the
    /// example above:
    ///
    /// ```rust
    /// use pavex::response::Response;
    ///     
    /// let response = Response::ok();
    /// ```
    ///
    /// Check out [`Response`]'s API documentation for a complete list of all
    /// the supported shorthands.
    pub fn new(status_code: StatusCode) -> Self {
        let inner = http::Response::new(Empty::new());
        Self { inner }.set_status(status_code)
    }
}

impl<Body> Response<Body> {
    /// Change the status code of the [`Response`].
    ///
    /// # Example
    ///
    /// ```rust
    /// use pavex::http::StatusCode;
    /// use pavex::response::Response;
    ///
    /// let mut response = Response::ok();
    /// assert_eq!(response.status(), StatusCode::OK);
    ///
    /// // Change the status code to `CREATED`.
    /// response = response.set_status(StatusCode::CREATED);
    /// assert_eq!(response.status(), StatusCode::CREATED);
    /// ```
    pub fn set_status(mut self, status: StatusCode) -> Self {
        *self.inner.status_mut() = status;
        self
    }

    /// Change the HTTP version of the [`Response`].
    ///
    /// # Example
    ///
    /// ```rust
    /// use pavex::http::Version;
    /// use pavex::response::Response;
    ///
    /// let mut response = Response::ok();
    /// // By default, the HTTP version is HTTP/1.1.
    /// assert_eq!(response.version(), Version::HTTP_11);
    ///
    /// // Change the HTTP version to HTTP/2.
    /// response = response.set_version(Version::HTTP_2);
    /// assert_eq!(response.version(), Version::HTTP_2);
    /// ```
    pub fn set_version(mut self, version: crate::http::Version) -> Self {
        *self.inner.version_mut() = version;
        self
    }

    /// Append a value to a [`Response`] header.
    ///
    /// If the header is not present, it is added with the given value.  
    /// If the header is present, the value is appended to the end
    /// of the comma-separated list of existing values for that header.
    ///
    /// # Example
    ///
    /// ```rust
    /// use pavex::http::{header::HOST, HeaderValue};
    /// use pavex::response::Response;
    ///
    /// let mut response = Response::ok();
    /// assert!(response.headers().get("host").is_none());
    ///
    /// // Append a value to the `host` header.
    /// let value = HeaderValue::from_static("world");
    /// response = response.append_header(HOST, value);
    ///
    /// let headers: Vec<_> = response.headers().get_all("host").iter().collect();
    /// assert_eq!(headers.len(), 1);
    /// assert_eq!(headers[0], "world");
    ///
    /// // Append another value to the `host` header.
    /// let value = HeaderValue::from_static("earth");
    /// response = response.append_header(HOST, value);
    ///
    /// let headers: Vec<_> = response.headers().get_all("host").iter().collect();
    /// assert_eq!(headers.len(), 2);
    /// assert_eq!(headers[0], "world");
    /// assert_eq!(headers[1], "earth");
    /// ```
    ///
    /// # Alternatives
    ///
    /// If you want to replace the value of a header instead of appending to it,
    /// use [`insert_header`](Response::insert_header) instead.
    pub fn append_header(
        mut self,
        key: crate::http::HeaderName,
        value: crate::http::HeaderValue,
    ) -> Self {
        self.inner.headers_mut().append(key, value);
        self
    }

    /// Insert a header value into the [`Response`].
    ///
    /// If the header key is not present, it is added with the given value.
    /// If the header key is present, its value is replaced with the given value.
    ///
    /// # Example
    ///
    /// ```rust
    /// use pavex::http::{header::HOST, HeaderValue};
    /// use pavex::response::Response;
    ///     
    /// let mut response = Response::ok();
    /// assert!(response.headers().get("host").is_none());
    ///     
    /// // Insert a value into the `host` header.
    /// let value = HeaderValue::from_static("world");
    /// response = response.insert_header(HOST, value);
    ///
    /// let headers: Vec<_> = response.headers().get_all("host").iter().collect();
    /// assert_eq!(headers.len(), 1);
    /// assert_eq!(headers[0], "world");
    ///
    /// // Insert another value into the `host` header.
    /// let value = HeaderValue::from_static("earth");
    /// response = response.insert_header(HOST, value);
    ///     
    /// let headers: Vec<_> = response.headers().get_all("host").iter().collect();
    /// assert_eq!(headers.len(), 1);
    /// assert_eq!(headers[0], "earth");
    /// ```
    ///
    /// # Alternatives
    ///
    /// If you want to append to the current header value instead of replacing it,
    /// use [`append_header`](Response::append_header) instead.
    pub fn insert_header(
        mut self,
        key: crate::http::HeaderName,
        value: crate::http::HeaderValue,
    ) -> Self {
        self.inner.headers_mut().insert(key, value);
        self
    }

    /// Set the [`Response`] body.
    ///
    /// The provided body must implement the [`TypedBody`] trait.  
    /// The `Content-Type` header is automatically set to the value returned
    /// by [`TypedBody::content_type`].
    ///
    /// If a body is already set, it is replaced.
    ///
    /// # Example
    ///
    /// ```rust
    /// use pavex::response::{Response, body::Html};
    /// use pavex::http::header::CONTENT_TYPE;
    ///
    /// let typed_body = "Hello, world!";
    /// let response = Response::ok().set_typed_body(typed_body);
    ///
    /// // The `Content-Type` header is set automatically
    /// // when using `set_typed_body`.
    /// assert_eq!(response.headers()[CONTENT_TYPE], "text/plain; charset=utf-8");
    /// ```
    ///
    /// # Built-in `TypedBody` implementations
    ///
    /// Pavex provides several implementations of [`TypedBody`] out of the box,
    /// to cover the most common use cases:
    ///
    /// - [`String`](std::string::String), [`&'static str`](std::primitive::str)
    ///   and [`Cow<'static, str>`](std::borrow::Cow) for `text/plain; charset=utf-8` responses.
    /// - [`Vec<u8>`](std::vec::Vec), [`&'static [u8]`](std::primitive::u8),
    ///  [`Cow<'static, [u8]>`](std::borrow::Cow) and [`Bytes`](bytes::Bytes) for `application/octet-stream` responses.
    /// - [`Json`](crate::response::body::Json) for `application/json` responses.
    /// - [`Html`](crate::response::body::Html) for `text/html; charset=utf-8` responses.
    ///
    /// Check out the [`body`](super::body) sub-module for an exhaustive list.
    ///
    /// # Raw body
    ///
    /// If you don't want Pavex to automatically set the `Content-Type` header,
    /// you might want to use [`Response::set_raw_body`] instead.
    pub fn set_typed_body<NewBody>(self, body: NewBody) -> Response<<NewBody as TypedBody>::Body>
    where
        NewBody: TypedBody,
    {
        let (mut head, _) = self.inner.into_parts();
        head.headers.insert(CONTENT_TYPE, body.content_type());
        http::Response::from_parts(head, body.body()).into()
    }

    /// Set the body of the [`Response`] to the given value, without setting
    /// the `Content-Type` header.
    ///
    /// This method should only be used if you need fine-grained control over
    /// the `Content-Type` header or the body type. In all other circumstances, use
    /// [`set_typed_body`](Response::set_typed_body).
    ///
    /// # Example
    ///
    /// ```rust
    /// use pavex::response::Response;
    /// use pavex::response::body::raw::{Bytes, Full};
    /// use pavex::http::header::CONTENT_TYPE;
    ///     
    /// let raw_body: Full<Bytes> = Full::new("Hello, world!".into());
    /// let response = Response::ok().set_raw_body(raw_body);
    ///
    /// // The `Content-Type` header is not set automatically
    /// // when using `set_raw_body`.
    /// assert_eq!(response.headers().get(CONTENT_TYPE), None);
    /// ```
    pub fn set_raw_body<NewBody>(self, body: NewBody) -> Response<NewBody>
    where
        NewBody: http_body::Body<Data = Bytes> + Send + Sync + 'static,
    {
        let (head, _) = self.inner.into_parts();
        http::Response::from_parts(head, body).into()
    }

    /// Box the current [`Response`] body.
    ///
    /// This can be useful when:
    ///
    /// - you need to return a [`Response`] with a body that implements [`http_body::Body`]
    ///   but you don't know the exact type of the body at compile time.
    ///
    /// - you need to return the same type of body from different branches of an `if` or `match`
    ///   statement.
    ///
    /// # Example
    ///
    /// ```rust
    /// use pavex::response::Response;
    ///
    /// # let user_exists = true;
    /// // The call to `.box_body` is necessary here because the two branches
    /// // of the `if` statement return different types of body: one returns
    /// // `Full<Bytes>` and the other returns `Empty<Bytes>`.
    /// // The compiler would reject this code without the call to `.box_body`.
    /// let response = if user_exists {
    ///    Response::ok().set_typed_body("Hey there!").box_body()
    /// } else {
    ///    Response::not_found().box_body()
    /// };
    /// ```
    pub fn box_body(self) -> Response<BoxBody>
    where
        Body: http_body::Body<Data = Bytes> + Send + Sync + 'static,
        Body::Error: Into<Box<dyn std::error::Error + Send + Sync>> + 'static,
    {
        let (head, body) = self.inner.into_parts();
        http::Response::from_parts(head, boxed(body)).into()
    }

    /// Get a mutable reference to the [`Response`] body.
    pub fn body_mut(&mut self) -> &mut Body {
        self.inner.body_mut()
    }

    /// Get a mutable reference to the [`Response`] headers.
    ///
    /// # Example
    ///
    /// ```rust
    /// use pavex::response::Response;
    /// use pavex::http::{header::CONTENT_TYPE, HeaderValue};
    /// use mime::TEXT_PLAIN_UTF_8;
    ///
    /// let mut response = Response::ok();
    ///
    /// // Get a mutable reference to the headers.
    /// let headers = response.headers_mut();
    ///
    /// // Insert a header.
    /// let value = HeaderValue::from_static(TEXT_PLAIN_UTF_8.as_ref());
    /// headers.insert(CONTENT_TYPE, value);
    ///
    /// assert_eq!(headers.len(), 1);
    ///
    /// // Remove a header.
    /// headers.remove(CONTENT_TYPE);
    ///
    /// assert!(headers.is_empty());
    /// ```
    pub fn headers_mut(&mut self) -> &mut crate::http::HeaderMap {
        self.inner.headers_mut()
    }
}

impl<Body> Response<Body> {
    /// Get a reference to the [`Response`] status code.
    ///
    /// # Example
    ///
    /// ```rust
    /// use pavex::{http::StatusCode, response::Response};
    ///
    /// let response = Response::bad_request();
    /// assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    /// ```
    ///
    /// # Mutation
    ///
    /// Check out [`Response::set_status`] if you need to modify the
    /// status code of the [`Response`].
    pub fn status(&self) -> StatusCode {
        self.inner.status()
    }

    /// Get a reference to the version of the HTTP protocol used by the [`Response`].
    ///
    /// # Example
    ///
    /// ```rust
    /// use pavex::http::Version;
    /// use pavex::response::Response;
    ///
    /// let mut response = Response::ok();
    /// // By default, the HTTP version is HTTP/1.1.
    /// assert_eq!(response.version(), Version::HTTP_11);
    /// ```
    ///
    /// # Mutation
    ///
    /// Check out [`Response::set_version`] if you need to modify the
    /// HTTP protocol version used by the [`Response`].
    pub fn version(&self) -> crate::http::Version {
        self.inner.version()
    }

    /// Get a reference to the [`Response`] headers.
    ///
    /// # Example
    ///
    /// ```rust
    /// use pavex::http::{header::{HOST, SERVER}, HeaderValue};
    /// use pavex::response::Response;
    ///     
    /// let response = Response::ok()
    ///     .append_header(HOST, HeaderValue::from_static("world"))
    ///     .append_header(HOST, HeaderValue::from_static("earth"))
    ///     .insert_header(SERVER, HeaderValue::from_static("Pavex"));
    ///
    /// let headers = response.headers();
    /// assert_eq!(headers.len(), 3);
    ///
    /// let host_values: Vec<_> = response.headers().get_all("host").iter().collect();
    /// assert_eq!(host_values.len(), 2);
    /// assert_eq!(host_values[0], "world");
    /// assert_eq!(host_values[1], "earth");
    ///
    /// assert_eq!(headers[SERVER], "Pavex");
    /// ```
    ///
    /// # Mutation
    ///
    /// If you need to modify the [`Response`] headers, check out:
    ///
    /// - [`Response::append_header`]
    /// - [`Response::insert_header`]
    /// - [`Response::headers_mut`]
    pub fn headers(&self) -> &crate::http::HeaderMap {
        self.inner.headers()
    }

    /// Get a reference to the [`Response`] body.
    ///
    /// # Mutation
    ///
    /// If you need to modify the [`Response`] body, check out:
    ///
    /// - [`Response::set_typed_body`]
    /// - [`Response::set_raw_body`]
    /// - [`Response::body_mut`]
    pub fn body(&self) -> &Body {
        self.inner.body()
    }
}

impl<Body> Response<Body> {
    /// Break down the [`Response`] into its two components: the [`ResponseHead`]
    /// and the body.
    ///
    /// This method consumes the [`Response`].
    ///
    /// # Related
    ///
    /// You can use [`Response::from_parts`] to reconstruct a [`Response`] from
    /// a [`ResponseHead`] and a body.
    pub fn into_parts(self) -> (ResponseHead, Body) {
        let (head, body) = self.inner.into_parts();
        (head.into(), body)
    }

    /// Build a [`Response`] from its two components: the [`ResponseHead`]
    /// and the body.
    ///
    /// # Related
    ///
    /// You can use [`Response::into_parts`] to decompose a [`Response`] from
    /// a [`ResponseHead`] and a body.
    pub fn from_parts(head: ResponseHead, body: Body) -> Self {
        Self {
            inner: http::Response::from_parts(head.into(), body),
        }
    }
}

impl<Body> From<http::Response<Body>> for Response<Body> {
    fn from(inner: http::Response<Body>) -> Self {
        Self { inner }
    }
}

impl<Body> From<Response<Body>> for http::Response<Body> {
    fn from(res: Response<Body>) -> Self {
        res.inner
    }
}

impl From<ResponseHead> for http::response::Parts {
    fn from(head: ResponseHead) -> Self {
        let ResponseHead {
            status,
            version,
            headers,
        } = head;
        // Is there no better way to do create a new `Parts` instance?
        let (mut parts, _) = http::response::Response::builder()
            .body(Empty::<()>::new())
            .unwrap()
            .into_parts();
        parts.status = status;
        parts.version = version;
        parts.headers = headers;
        parts
    }
}

impl From<http::response::Parts> for ResponseHead {
    fn from(parts: http::response::Parts) -> Self {
        let http::response::Parts {
            status,
            version,
            headers,
            ..
        } = parts;
        Self {
            status,
            version,
            headers,
        }
    }
}

macro_rules! shorthand {
    ($name:ident) => {
        paste::paste! {
            #[doc = "Start building a new [`Response`] with [`" $name "`](`StatusCode::" $name "`) as status code."]
            pub fn [<$name:lower>]() -> Response<Empty<Bytes>> {
                Response::new(StatusCode::[<$name>])
            }
        }
    };
}

/// Shorthand for building a new [`Response`] using a well-known status code.
impl Response<Empty<Bytes>> {
    /// Start building a new [`Response`] with [`CONTINUE`](StatusCode::CONTINUE) as status code.
    // This is special-cased because `continue` is a keyword in Rust.
    pub fn continue_() -> Response<Empty<Bytes>> {
        Response::new(StatusCode::CONTINUE)
    }

    // 2xx
    shorthand!(SWITCHING_PROTOCOLS);
    shorthand!(PROCESSING);
    shorthand!(OK);
    shorthand!(CREATED);
    shorthand!(ACCEPTED);
    shorthand!(NON_AUTHORITATIVE_INFORMATION);

    shorthand!(NO_CONTENT);
    shorthand!(RESET_CONTENT);
    shorthand!(PARTIAL_CONTENT);
    shorthand!(MULTI_STATUS);
    shorthand!(ALREADY_REPORTED);

    // 3xx
    shorthand!(MULTIPLE_CHOICES);
    shorthand!(MOVED_PERMANENTLY);
    shorthand!(FOUND);
    shorthand!(SEE_OTHER);
    shorthand!(NOT_MODIFIED);
    shorthand!(USE_PROXY);
    shorthand!(TEMPORARY_REDIRECT);
    shorthand!(PERMANENT_REDIRECT);

    // 4xx
    shorthand!(BAD_REQUEST);
    shorthand!(NOT_FOUND);
    shorthand!(UNAUTHORIZED);
    shorthand!(PAYMENT_REQUIRED);
    shorthand!(FORBIDDEN);
    shorthand!(METHOD_NOT_ALLOWED);
    shorthand!(NOT_ACCEPTABLE);
    shorthand!(PROXY_AUTHENTICATION_REQUIRED);
    shorthand!(REQUEST_TIMEOUT);
    shorthand!(CONFLICT);
    shorthand!(GONE);
    shorthand!(LENGTH_REQUIRED);
    shorthand!(PRECONDITION_FAILED);
    shorthand!(PRECONDITION_REQUIRED);
    shorthand!(PAYLOAD_TOO_LARGE);
    shorthand!(URI_TOO_LONG);
    shorthand!(UNSUPPORTED_MEDIA_TYPE);
    shorthand!(RANGE_NOT_SATISFIABLE);
    shorthand!(EXPECTATION_FAILED);
    shorthand!(UNPROCESSABLE_ENTITY);
    shorthand!(TOO_MANY_REQUESTS);
    shorthand!(REQUEST_HEADER_FIELDS_TOO_LARGE);
    shorthand!(UNAVAILABLE_FOR_LEGAL_REASONS);

    // 5xx
    shorthand!(INTERNAL_SERVER_ERROR);
    shorthand!(NOT_IMPLEMENTED);
    shorthand!(BAD_GATEWAY);
    shorthand!(SERVICE_UNAVAILABLE);
    shorthand!(GATEWAY_TIMEOUT);
    shorthand!(HTTP_VERSION_NOT_SUPPORTED);
    shorthand!(VARIANT_ALSO_NEGOTIATES);
    shorthand!(INSUFFICIENT_STORAGE);
    shorthand!(LOOP_DETECTED);
}
