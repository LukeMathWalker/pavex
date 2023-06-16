use super::TypedBody;
use crate::http::HeaderValue;
use bytes::Bytes;
use http_body::Full;
use mime::APPLICATION_JSON;

/// A [`Response`](crate::response::Response) body with `Content-Type` set to
/// `application/json`.
///
/// # Example
///
/// ```rust
/// use pavex::response::{Response, body::Json};
/// use pavex::http::header::CONTENT_TYPE;
///
/// #[derive(serde::Serialize)]
/// struct HomeDetails {
///     city: String,
///     postcode: String,
/// }
///
/// let details = HomeDetails {
///     city: "London".into(),
///     postcode: "N5 2EF".into(),
/// };
/// let json = Json::new(details).expect("Failed to serialize the response body");
/// let response = Response::ok().set_typed_body(json);
///
/// assert_eq!(response.headers()[CONTENT_TYPE], "application/json");
/// ```
pub struct Json(Bytes);

impl Json {
    /// Build a new [`Json`] response by serializing to JSON an instance of type `T`.
    ///
    /// It returns an error if serialization fails.
    pub fn new<T>(value: T) -> Result<Self, JsonSerializationError>
    where
        T: serde::Serialize,
    {
        let bytes = serde_json::to_vec(&value).map_err(JsonSerializationError)?;
        Ok(Self(bytes.into()))
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
/// The error returned by [`Json::new`] when the serialization into JSON fails.
pub struct JsonSerializationError(serde_json::Error);

impl TypedBody for Json {
    type Body = Full<Bytes>;

    fn content_type(&self) -> HeaderValue {
        HeaderValue::from_static(APPLICATION_JSON.as_ref())
    }

    fn body(self) -> Self::Body {
        Full::new(self.0)
    }
}
