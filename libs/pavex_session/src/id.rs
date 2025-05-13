#[derive(
    Clone, Copy, Eq, PartialEq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(transparent)]
/// The identifier for a session.
///
/// # Format stability
///
/// The session ID is guaranteed to be a valid UUID.
/// The format of the UUID is not guaranteed to be stable across different versions of this library.
///
/// It is recommended to treat the session ID as an opaque value in your application.
/// Knowing the format is primarily useful when implementing custom session storage backends, as
/// it allows you to leverage optimizations in your data store that are specific to the UUID format
/// (e.g. a dedicated data type, such as `UUID` in PostgreSQL).
pub struct SessionId(uuid::Uuid);

impl SessionId {
    /// Generate a new random identifier using the random number generator
    /// provided by the underlying operating system.
    pub fn random() -> Self {
        Self(uuid::Uuid::new_v4())
    }

    /// Returns the inner `uuid::Uuid` value.
    pub fn inner(&self) -> uuid::Uuid {
        self.0
    }

    pub fn as_bytes(&self) -> &[u8; 16] {
        self.0.as_bytes()
    }
}
