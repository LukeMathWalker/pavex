#[derive(
    Debug, Clone, Copy, Eq, PartialEq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(transparent)]
/// The identifier for a session.
///
/// # Format stability
///
/// From an API perspective, a session id is an opaque sequence of bytes.  
/// Do **not** depend on the specifics of the underlying representation.
/// It may change between versions and those changes will not be considered
/// breaking changes.
pub struct SessionId(uuid::Uuid);

impl SessionId {
    /// Generate a new random identifier using the random number generator
    /// provided by the underlying operating system.
    pub fn random() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}
