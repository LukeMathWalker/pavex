use crate::http::HeaderValue;
use pavex_macros::methods;
use std::fmt::Formatter;
use std::hash::{Hash, Hasher};
use type_safe_id::{StaticType, TypeSafeId};
use uuid::Uuid;

/// A unique identifier generated for each incoming request.
///
/// It is a [TypeId](https://github.com/jetify-com/typeid).
/// The prefix is always `sri`, followed by an underscore and UUIDv7 in base32 encoding.
/// For example:
///
/// ```text
///  sri_2x4y6z8a0b1c2d3e4f5g6h7j8k
///  └─┘ └────────────────────────┘
/// prefix    uuid suffix (base32)
/// ```
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
#[derive(Debug, Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
pub struct ServerRequestId(TypeSafeId<SriTag>);

impl Hash for ServerRequestId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.uuid().hash(state)
    }
}

#[derive(Default, Debug, Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct SriTag;

impl StaticType for SriTag {
    const TYPE: &'static str = "sri";
}

#[methods]
impl ServerRequestId {
    /// The default constructor for [`ServerRequestId`].
    ///
    /// It generates a new request id using a [UUID v7](https://datatracker.ietf.org/doc/html/draft-peabody-dispatch-new-uuid-format-04#name-uuid-version-7),
    /// with a random number and the current (UNIX) timestamp.
    #[request_scoped]
    pub fn generate() -> Self {
        Self(TypeSafeId::from_uuid(Uuid::now_v7()))
    }

    /// Access the underlying UUID.
    pub fn inner(&self) -> Uuid {
        self.0.uuid()
    }

    /// Return a [hyphenated representation](https://docs.rs/uuid/1.7.0/uuid/struct.Uuid.html#formatting)
    /// of [`ServerRequestId`] to be used as a [`HeaderValue`].
    pub fn header_value(&self) -> HeaderValue {
        // It is always a valid header value.
        self.0.to_string().try_into().unwrap()
    }
}

impl From<Uuid> for ServerRequestId {
    fn from(value: Uuid) -> Self {
        Self(TypeSafeId::from_uuid(value))
    }
}

impl std::fmt::Display for ServerRequestId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_request_id() {
        let request_id = ServerRequestId::generate();
        let header_value = request_id.header_value();
        let header_value = header_value.to_str().unwrap();
        assert!(header_value.starts_with("sri_"));
        // Prefix + `_` + UUID in base32 encoding
        assert_eq!(header_value.len(), 3 + 1 + 26)
    }
}
