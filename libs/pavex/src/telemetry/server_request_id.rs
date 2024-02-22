use crate::blueprint::constructor::{Constructor, Lifecycle, RegisteredConstructor};
use crate::blueprint::Blueprint;
use crate::f;
use crate::http::HeaderValue;
use std::fmt::Formatter;
use uuid::Uuid;

/// A unique identifier generated for each incoming request.
///
/// # Example
///
/// ```rust
/// use http::{HeaderName, HeaderValue};
/// use pavex::response::Response;
/// use pavex::telemetry::ServerRequestId;
///
/// pub async fn request_handler(request_id: ServerRequestId) -> Response {
///     // Add the request id as a header on the outgoing response
///     Response::ok()
///         .insert_header(
///             HeaderName::from_static("X-Request-Id"),
///             request_id.header_value()
///         )
/// }
/// ```
#[derive(Debug, Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ServerRequestId(Uuid);

impl ServerRequestId {
    /// The default constructor for [`ServerRequestId`].
    ///
    /// It generates a new request id using a [UUID v7](https://datatracker.ietf.org/doc/html/draft-peabody-dispatch-new-uuid-format-04#name-uuid-version-7),
    /// with a random number and the current (UNIX) timestamp.
    pub fn generate() -> Self {
        Self(Uuid::now_v7())
    }

    /// Access the underlying UUID.
    pub fn inner(&self) -> Uuid {
        self.0
    }

    /// Return a [hyphenated representation](https://docs.rs/uuid/1.7.0/uuid/struct.Uuid.html#formatting)
    /// of [`ServerRequestId`] to be used as a [`HeaderValue`].
    pub fn header_value(&self) -> HeaderValue {
        // The hyphenated representation of a UUID is always a valid header value.
        format!("{self}").try_into().unwrap()
    }
}

impl From<Uuid> for ServerRequestId {
    fn from(value: Uuid) -> Self {
        Self(value)
    }
}

impl std::fmt::Display for ServerRequestId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.0)
    }
}

/// Registration helpers.
impl ServerRequestId {
    /// Register the [default constructor](Self::generate)
    /// for [`ServerRequestId`] with a [`Blueprint`].
    pub fn register(bp: &mut Blueprint) -> RegisteredConstructor {
        Self::default_constructor().register(bp)
    }

    /// The [default constructor](Self::generate) for [`ServerRequestId`].
    pub fn default_constructor() -> Constructor {
        Constructor::new(
            f!(pavex::telemetry::ServerRequestId::generate),
            Lifecycle::RequestScoped,
        )
    }
}
