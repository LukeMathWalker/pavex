#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
/// Configure the way session state is stored.
pub struct SessionStateConfig {
    /// The time-to-live of the server session state, i.e.
    /// how long to keep the server-side state of a session
    /// in the storage backend you chose.
    ///
    /// This value is also used to control the expiration
    /// of the client-side session cookie if [`SessionCookieConfig::kind`]
    /// is set to [`SessionCookieKind::Persistent`].
    ///
    /// # Default
    ///
    /// The default value is 24 hours.
    ///
    /// [`SessionCookieConfig::kind`]: super::SessionCookieConfig::kind
    /// [`SessionCookieKind::Persistent`]: super::SessionCookieKind::Persistent
    #[serde(with = "humantime_serde", default = "default_ttl")]
    pub ttl: std::time::Duration,
    /// The event that triggers the extension of the time-to-live
    /// of the current session.
    #[serde(default)]
    pub extend_ttl: TtlExtensionTrigger,
    /// The server will skip TTL extension if the remaining TTL
    /// is greater than this threshold.  
    /// The threshold is a ratio between 0 and 1, interpreted as
    /// a percentage of the total TTL.
    ///
    /// If set to `None`, the server will never skip TTL extension.
    ///
    /// # Performance impact
    ///
    /// Setting a sensible threshold will:
    ///
    /// - reduce the number of requests to your storage backend
    /// - reduce your server latency by removing a network request from the critical path
    ///
    /// # Default
    ///
    /// By default, the threshold is set to 0.8—i.e. 80% of the total TTL.  
    ///
    /// # Example
    ///
    /// Let's assume that the TTL for a new session is set to 24 hours.  
    /// With the default threshold of 0.8, the server will skip TTL extension requests
    /// if the remaining session TTL is greater than 19.2 hours.  
    /// In other words, the server expects at most ~0.2 TTL extension requests per hour for
    /// each active session, regardless of the number of requests the server receives
    /// for that session.
    #[serde(default = "default_ttl_extension_threshold")]
    pub ttl_extension_threshold: Option<TtlExtensionThreshold>,
    /// Determines when the storage backend should be asked to create a new session state record.
    #[serde(default)]
    pub server_state_creation: ServerStateCreation,
}

impl Default for SessionStateConfig {
    fn default() -> Self {
        Self {
            ttl: default_ttl(),
            extend_ttl: Default::default(),
            ttl_extension_threshold: default_ttl_extension_threshold(),
            server_state_creation: Default::default(),
        }
    }
}

fn default_ttl() -> std::time::Duration {
    std::time::Duration::from_secs(60 * 60 * 24)
}

fn default_ttl_extension_threshold() -> Option<TtlExtensionThreshold> {
    Some(TtlExtensionThreshold::new(0.8).unwrap())
}

#[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
/// Configure when the TTL for an existing session should be extended.
pub enum TtlExtensionTrigger {
    /// The TTL of the current session is refreshed on every request where the
    /// server either:
    ///
    /// - Modified the session state
    /// - Loaded the server state from the storage backend
    ///
    /// This is the default.
    ///
    /// # Performance impact
    ///
    /// TTL refreshes are not free, as they require an additional
    /// request to the storage backend if the state was otherwise unchanged.
    /// This impacts both the total load on your storage backend
    /// (i.e. number of queries it has to handle) and the latency of the requests served by your server.
    ///
    /// This impact can be mitigated by setting a [TTL extension threshold](SessionStateConfig::ttl_extension_threshold).
    #[default]
    OnStateLoadsAndChanges,
    /// The TTL of the current session is only refreshed when the session state is modified.
    ///
    /// It doesn't distinguish between changes to the client-side and the server-side session state
    /// when determining if the TTL should be refreshed.
    ///
    /// # Performance impact
    ///
    /// [`OnStateChanges`] may reduce the number of requests to the storage backend
    /// compared to [`OnStateLoadsAndChanges`], as well as improve the latency of the requests served
    /// by your server by removing a network request from the critical path.  
    /// It primarily depends on the
    /// [TTL extension threshold](SessionStateConfig::ttl_extension_threshold) you set, if any.
    ///
    /// [`OnStateChanges`]: TtlExtensionTrigger::OnStateChanges
    /// [`OnStateLoadsAndChanges`]: TtlExtensionTrigger::OnStateLoadsAndChanges
    OnStateChanges,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
/// Configure when the storage backend should be asked to create a new session state record.
///
/// Regardless of the policy you choose, remember that no session record will be created if:
///
/// - There is no client-side session state (i.e. the client didn't send a session cookie)
/// - The client-side session state is empty
/// - The server-side session state is empty
pub enum ServerStateCreation {
    /// The storage backend won't be asked to create a new session state
    /// record if the server-side state is empty.
    SkipIfEmpty,
    /// The storage backend will always be asked to create a server-side session state
    /// record if a client-side session state is present.
    ///
    /// This is the default policy.
    #[default]
    NeverSkip,
}

#[derive(Debug, Clone, Copy, serde::Serialize)]
/// A ratio between 0 and 1, interpreted as a percentage of the time-to-live of a fresh session.
pub struct TtlExtensionThreshold(f32);

impl<'de> serde::Deserialize<'de> for TtlExtensionThreshold {
    fn deserialize<D>(deserializer: D) -> Result<TtlExtensionThreshold, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = f32::deserialize(deserializer)?;
        TtlExtensionThreshold::new(value).map_err(serde::de::Error::custom)
    }
}

impl TtlExtensionThreshold {
    pub fn new(value: f32) -> Result<Self, InvalidTtlExtensionThreshold> {
        if value < 0.0 || value > 1.0 {
            Err(InvalidTtlExtensionThreshold(value))
        } else {
            Ok(Self(value))
        }
    }

    pub fn inner(self) -> f32 {
        self.0
    }
}

impl TryFrom<f32> for TtlExtensionThreshold {
    type Error = InvalidTtlExtensionThreshold;

    fn try_from(value: f32) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<f64> for TtlExtensionThreshold {
    type Error = InvalidTtlExtensionThreshold;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        Self::new(value as f32)
    }
}

#[derive(Debug)]
/// Error raised when trying to create a [`TtlExtensionThreshold`] with an invalid value.
pub struct InvalidTtlExtensionThreshold(f32);

impl std::fmt::Display for InvalidTtlExtensionThreshold {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "TTL extension threshold must be a ratio between 0 and 1, got {}",
            self.0
        )
    }
}

impl std::error::Error for InvalidTtlExtensionThreshold {}
