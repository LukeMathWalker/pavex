use anyhow::Context;
use pavex::blueprint::{
    Blueprint, config::ConfigType, constructor::Constructor, linter::Lint,
    middleware::PostProcessingMiddleware,
};
use pavex::f;
use pavex_session::{
    SessionId,
    store::{
        SessionRecord, SessionRecordRef, SessionStorageBackend,
        errors::{
            ChangeIdError, CreateError, DeleteError, DeleteExpiredError, DuplicateIdError,
            LoadError, UnknownIdError, UpdateError, UpdateTtlError,
        },
    },
};
use redis::{AsyncCommands, ExistenceCheck, SetExpiry, SetOptions, Value, aio::ConnectionManager};
use serde;
use std::num::NonZeroUsize;

#[derive(Clone, Debug, Default, serde::Deserialize)]
pub struct RedisSessionStoreConfig {
    #[serde(default)]
    pub namespace: Option<String>,
}

#[derive(Clone)]
/// A server-side session store using Redis as its backend.
///
/// # Implementation details
pub struct RedisSessionStore {
    conn: ConnectionManager,
    cfg: RedisSessionStoreConfig,
}

impl std::fmt::Debug for RedisSessionStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RedisSessionStore")
            .field("conn", &"<ConnectionManager>")
            .field("cfg", &format_args!("{:?}", self.cfg))
            .finish()
    }
}

impl RedisSessionStore {
    /// Creates a new Redis session store instance.
    ///
    /// It requires a redis::ConnectionManager instance to interact with redis.
    pub fn new(conn: ConnectionManager, cfg: RedisSessionStoreConfig) -> Self {
        Self { conn, cfg }
    }

    fn redis_key(&self, id: &SessionId) -> String {
        if let Some(namespace) = &self.cfg.namespace {
            format!("{}:{}", namespace, id.inner().to_string())
        } else {
            id.inner().to_string()
        }
    }
}

fn err_unknown(id: &SessionId) -> UnknownIdError {
    UnknownIdError { id: id.to_owned() }
}

fn err_duplicate(id: &SessionId) -> DuplicateIdError {
    DuplicateIdError { id: id.to_owned() }
}

fn redis_value_type_name(value: Value) -> &'static str {
    match value {
        Value::Nil => "Nil",
        Value::Okay => "Okay",
        Value::Int(_) => "Int",
        Value::BulkString(_) => "BulkString",
        Value::Array(_) => "Array",
        Value::SimpleString(_) => "SimpleString",
        Value::Map(_) => "Map",
        Value::Set(_) => "Set",
        Value::Attribute { .. } => "Attribute",
        Value::Double(_) => "Double",
        Value::Boolean(_) => "Boolean",
        Value::VerbatimString { .. } => "VerbatimString",
        Value::BigNumber(_) => "BigNumber",
        Value::Push { .. } => "Push",
        Value::ServerError(_) => "ServerError",
    }
}

#[async_trait::async_trait]
impl SessionStorageBackend for RedisSessionStore {
    /// Creates a new session record in the store using the provided ID.
    #[tracing::instrument(name = "Create server-side session record", level = tracing::Level::INFO, skip_all)]
    async fn create(
        &self,
        id: &SessionId,
        record: SessionRecordRef<'_>,
    ) -> Result<(), CreateError> {
        match self
            .conn
            .clone()
            .set_options(
                self.redis_key(id),
                serde_json::to_vec(&record.state)?,
                SetOptions::default()
                    .conditional_set(ExistenceCheck::NX)
                    .with_expiration(SetExpiry::EX(record.ttl.as_secs())),
            )
            .await
            .map_err(|e| CreateError::Other(e.into()))?
        {
            Value::Okay => Ok(()),
            Value::Nil => Err(err_duplicate(id).into()),
            val => Err(CreateError::Other(anyhow::anyhow!(
                "Redis SET replied with {:?}. Expected Okay or Nil",
                val
            ))),
        }
    }

    /// Update the state of an existing session in the store.
    ///
    /// It overwrites the existing record with the provided one.
    #[tracing::instrument(name = "Update server-side session record", level = tracing::Level::INFO, skip_all)]
    async fn update(
        &self,
        id: &SessionId,
        record: SessionRecordRef<'_>,
    ) -> Result<(), UpdateError> {
        match self
            .conn
            .clone()
            .set_options(
                self.redis_key(id),
                serde_json::to_vec(&record.state)?,
                SetOptions::default()
                    .conditional_set(ExistenceCheck::XX)
                    .with_expiration(SetExpiry::EX(record.ttl.as_secs())),
            )
            .await
            .map_err(|e| UpdateError::Other(e.into()))?
        {
            Value::Okay => Ok(()),
            Value::Nil => Err(err_unknown(id).into()),
            val => Err(UpdateError::Other(anyhow::anyhow!(
                "Redis SET returned {:?}. Expected Okay or Nil",
                val
            ))),
        }
    }

    /// Update the TTL of an existing session record in the store.
    ///
    /// It leaves the session state unchanged.
    #[tracing::instrument(name = "Update TTL for server-side session record", level = tracing::Level::INFO, skip_all)]
    async fn update_ttl(
        &self,
        id: &SessionId,
        ttl: std::time::Duration,
    ) -> Result<(), UpdateTtlError> {
        let k = self.redis_key(id);
        let mut conn = self.conn.clone();
        match redis::pipe()
            .cmd("EXISTS")
            .arg(&k)
            .cmd("EXPIRE")
            .arg(&k)
            .arg(ttl.as_secs())
            .query_async(&mut conn)
            .await
            .map_err(|e| UpdateTtlError::Other(e.into()))?
        {
            (1, 1) => Ok(()),
            (0, 0) => Err(err_unknown(id).into()),
            (1, 0) => Err(UpdateTtlError::Other(anyhow::anyhow!(
                "Session key exists but redis failed to update TTL"
            ))),
            (0, 1) => Err(UpdateTtlError::Other(anyhow::anyhow!(
                "Unexpected reply from redis: redis should not report succesfully setting ttl for non-existent key"
            ))),
            (v, w) => Err(UpdateTtlError::Other(anyhow::anyhow!(
                "Unexpected reply from redis: EXISTS and EXPIRE only return 0 or 1. EXIST returned {:?}, EXPIRE returned {:?}",
                v,
                w
            ))),
        }
    }

    /// Loads an existing session record from the store using the provided ID.
    ///
    /// If a session with the given ID exists, it is returned. If the session
    /// does not exist or has been invalidated (e.g., expired), `None` is
    /// returned.
    #[tracing::instrument(name = "Load server-side session record", level = tracing::Level::INFO, skip_all)]
    async fn load(&self, session_id: &SessionId) -> Result<Option<SessionRecord>, LoadError> {
        let mut conn = self.conn.clone();
        let k = self.redis_key(session_id);
        let (ttl_reply, get_reply): (Value, Value) = redis::pipe()
            .cmd("TTL")
            .arg(&k)
            .cmd("GET")
            .arg(&k)
            .query_async(&mut conn)
            .await
            .map_err(|e| LoadError::Other(e.into()))?;

        let ttl = match ttl_reply {
            Value::Int(s) if s >= 0 => std::time::Duration::from_secs(s as u64),
            Value::Int(-1) => {
                return Err(LoadError::Other(anyhow::anyhow!(
                    "Fatal session management error: no TTL set for this session."
                )));
            }
            Value::Int(-2) => return Ok(None),
            _ => {
                return Err(LoadError::Other(anyhow::anyhow!(
                    "Redis TTL returned {}. Expected integer >= -2",
                    redis_value_type_name(ttl_reply)
                )));
            }
        };

        let state = match get_reply {
            Value::BulkString(raw) => serde_json::from_slice(&raw)
                .context("Failed to deserialize the retrieved session state")
                .map_err(LoadError::DeserializationError)?,
            _ => {
                return Err(LoadError::Other(anyhow::anyhow!(
                    "Redis GET replied {}. Expected BulkString.",
                    redis_value_type_name(get_reply)
                )));
            }
        };

        Ok(Some(SessionRecord { ttl, state }))
    }

    /// Deletes a session record from the store using the provided ID.
    ///
    /// If the session exists, it is removed from the store.
    #[tracing::instrument(name = "Delete server-side session record", level = tracing::Level::INFO, skip_all)]
    async fn delete(&self, id: &SessionId) -> Result<(), DeleteError> {
        let ndeleted: u64 = self
            .conn
            .clone()
            .del(self.redis_key(id))
            .await
            .map_err(|e| DeleteError::Other(e.into()))?;

        if ndeleted == 1 {
            Ok(())
        } else if ndeleted == 0 {
            Err(err_unknown(id).into())
        } else {
            return Err(DeleteError::Other(anyhow::anyhow!(
                "Redis DEL replied {:?}. Expected 0 or 1.",
                ndeleted
            )));
        }
    }

    /// Change the session id associated with an existing session record.
    ///
    /// The server-side state is left unchanged.
    #[tracing::instrument(name = "Change id for server-side session record", level = tracing::Level::INFO, skip_all)]
    async fn change_id(&self, old_id: &SessionId, new_id: &SessionId) -> Result<(), ChangeIdError> {
        let nchanged: u64 = self
            .conn
            .clone()
            .rename_nx(self.redis_key(old_id), self.redis_key(new_id))
            .await
            .map_err(|e| ChangeIdError::Other(e.into()))?;

        if nchanged == 1 {
            Ok(())
        } else if nchanged == 0 {
            Err(err_duplicate(new_id).into())
        } else {
            return Err(ChangeIdError::Other(anyhow::anyhow!(
                "Redis RENAMENX replied {:?}. Expected 0 or 1",
                nchanged
            )));
        }
    }

    /// Deletes expired session records from the store.
    ///
    /// Redis handles the deletion of expired keys automatically, so this method always
    /// returns Ok(0).
    async fn delete_expired(
        &self,
        _batch_size: Option<NonZeroUsize>,
    ) -> Result<usize, DeleteExpiredError> {
        Ok(0)
    }
}

pub struct RedisSessionKit {
    pub session: Option<Constructor>,
    pub incoming_session: Option<Constructor>,
    pub session_config: Option<ConfigType>,
    pub redis_session_store: Option<Constructor>,
    pub session_store: Option<Constructor>,
    pub session_finalizer: Option<PostProcessingMiddleware>,
}

impl Default for RedisSessionKit {
    fn default() -> Self {
        Self::new()
    }
}

impl RedisSessionKit {
    pub fn new() -> Self {
        let pavex_session::SessionKit {
            session,
            session_config,
            session_finalizer,
            incoming_session,
            ..
        } = pavex_session::SessionKit::new();
        Self {
            session,
            incoming_session,
            session_config,
            session_finalizer,
            redis_session_store: Some(
                Constructor::singleton(f!(crate::RedisSessionStore::new)).ignore(Lint::Unused),
            ),
            session_store: Some(
                Constructor::singleton(f!(pavex_session::SessionStore::new::<
                    crate::RedisSessionStore,
                >))
                .ignore(Lint::Unused),
            ),
        }
    }

    pub fn register(self, bp: &mut Blueprint) -> RegisteredRedisSessionKit {
        let mut kit = pavex_session::SessionKit::new();
        kit.session = self.session;
        kit.incoming_session = self.incoming_session;
        kit.session_config = self.session_config;
        kit.session_finalizer = self.session_finalizer;
        kit.register(bp);
        if let Some(redis_session_store) = self.redis_session_store {
            redis_session_store.register(bp);
        }
        if let Some(session_store) = self.session_store {
            session_store.register(bp);
        }

        RegisteredRedisSessionKit {}
    }
}

#[derive(Clone, Debug)]
#[non_exhaustive]
/// The type returned by [`RedisSessionKit::register`].
pub struct RegisteredRedisSessionKit {}

#[cfg(test)]
mod tests {
    use super::*;
    use pavex_session::store::{SessionRecordRef, SessionStorageBackend};
    use std::borrow::Cow;
    use std::collections::HashMap;
    use std::time::Duration;

    async fn create_test_store() -> RedisSessionStore {
        let client = redis::Client::open("redis://localhost:6379").unwrap();
        let conn = tokio::time::timeout(
            Duration::from_secs(2),
            redis::aio::ConnectionManager::new(client),
        )
        .await
        .expect("Failed to connect to Redis within 2 seconds - is Redis running on localhost:6379?")
        .unwrap();

        // Use random namespace to avoid test collisions
        let config = RedisSessionStoreConfig {
            namespace: Some(format!("test_{}", uuid::Uuid::new_v4())),
        };

        RedisSessionStore::new(conn, config)
    }

    fn create_test_record(
        _ttl_secs: u64,
    ) -> (SessionId, HashMap<Cow<'static, str>, serde_json::Value>) {
        let session_id = SessionId::random();
        let mut state = HashMap::new();
        state.insert(
            Cow::Borrowed("user_id"),
            serde_json::Value::String("test-user-123".to_string()),
        );
        state.insert(
            Cow::Borrowed("login_time"),
            serde_json::Value::String("2024-01-01T00:00:00Z".to_string()),
        );
        state.insert(
            Cow::Borrowed("permissions"),
            serde_json::json!(["read", "write"]),
        );
        state.insert(
            Cow::Borrowed("metadata"),
            serde_json::json!({
                "ip": "192.168.1.1",
                "user_agent": "test-agent",
                "session_start": 1640995200
            }),
        );
        (session_id, state)
    }

    #[tokio::test]
    async fn test_create_and_load_roundtrip() {
        let store = create_test_store().await;
        let (session_id, state) = create_test_record(3600);

        let record = SessionRecordRef {
            state: Cow::Borrowed(&state),
            ttl: Duration::from_secs(3600),
        };

        // Create session
        store.create(&session_id, record).await.unwrap();

        // Load session
        let loaded = store.load(&session_id).await.unwrap();
        assert!(loaded.is_some());

        let loaded_record = loaded.unwrap();

        // Verify all data is preserved correctly by comparing with original
        for (key, expected_value) in &state {
            assert_eq!(
                loaded_record.state.get(key).unwrap(),
                expected_value,
                "Mismatch for key: {}",
                key
            );
        }

        // Verify we have the same number of keys
        assert_eq!(loaded_record.state.len(), state.len());

        // Verify TTL is reasonable (should be close to 3600 seconds)
        assert!(loaded_record.ttl.as_secs() > 3550);
        assert!(loaded_record.ttl.as_secs() <= 3600);
    }

    #[tokio::test]
    async fn test_update_roundtrip() {
        let store = create_test_store().await;
        let (session_id, mut state) = create_test_record(3600);

        let record = SessionRecordRef {
            state: Cow::Borrowed(&state),
            ttl: Duration::from_secs(3600),
        };

        // Create initial session
        store.create(&session_id, record).await.unwrap();

        // Update the state
        state.insert(
            Cow::Borrowed("updated_field"),
            serde_json::Value::String("new_value".to_string()),
        );
        state.insert(
            Cow::Borrowed("user_id"),
            serde_json::Value::String("updated-user-456".to_string()),
        );
        state.insert(
            Cow::Borrowed("new_metadata"),
            serde_json::json!({
                "last_action": "update_session",
                "timestamp": 1640995260,
                "complex_data": {
                    "nested": {
                        "deeply": ["nested", "array", 123, true]
                    }
                }
            }),
        );

        let updated_record = SessionRecordRef {
            state: Cow::Borrowed(&state),
            ttl: Duration::from_secs(7200),
        };

        // Update session
        store.update(&session_id, updated_record).await.unwrap();

        // Load and verify updates
        let loaded = store.load(&session_id).await.unwrap().unwrap();

        // Verify all updated data is preserved correctly by comparing with updated state
        for (key, expected_value) in &state {
            assert_eq!(
                loaded.state.get(key).unwrap(),
                expected_value,
                "Mismatch for updated key: {}",
                key
            );
        }

        // Verify we have the same number of keys
        assert_eq!(loaded.state.len(), state.len());

        // Verify TTL was updated
        assert!(loaded.ttl.as_secs() > 3600);
    }

    #[tokio::test]
    async fn test_ttl_expiry() {
        let store = create_test_store().await;
        let (session_id, state) = create_test_record(1);

        let record = SessionRecordRef {
            state: Cow::Borrowed(&state),
            ttl: Duration::from_secs(1), // Very short TTL
        };

        // Create session with short TTL
        store.create(&session_id, record).await.unwrap();

        // Session should exist immediately
        let loaded = store.load(&session_id).await.unwrap();
        assert!(loaded.is_some());

        // Wait for expiration
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Session should be expired and not loadable
        let expired = store.load(&session_id).await.unwrap();
        assert!(expired.is_none());
    }

    #[tokio::test]
    async fn test_update_ttl_roundtrip() {
        let store = create_test_store().await;
        let (session_id, state) = create_test_record(3600);

        let record = SessionRecordRef {
            state: Cow::Borrowed(&state),
            ttl: Duration::from_secs(3600),
        };

        // Create session
        store.create(&session_id, record).await.unwrap();

        // Update TTL only
        store
            .update_ttl(&session_id, Duration::from_secs(7200))
            .await
            .unwrap();

        // Verify TTL was updated but data preserved
        let loaded = store.load(&session_id).await.unwrap().unwrap();

        // Verify original data is preserved by comparing with original state
        for (key, expected_value) in &state {
            assert_eq!(
                loaded.state.get(key).unwrap(),
                expected_value,
                "Mismatch for key after TTL update: {}",
                key
            );
        }
        assert!(loaded.ttl.as_secs() > 3600);
    }

    #[tokio::test]
    async fn test_delete_roundtrip() {
        let store = create_test_store().await;
        let (session_id, state) = create_test_record(3600);

        let record = SessionRecordRef {
            state: Cow::Borrowed(&state),
            ttl: Duration::from_secs(3600),
        };

        // Create session
        store.create(&session_id, record).await.unwrap();

        // Verify it exists
        let loaded = store.load(&session_id).await.unwrap();
        assert!(loaded.is_some());

        // Delete session
        store.delete(&session_id).await.unwrap();

        // Verify it's gone
        let deleted = store.load(&session_id).await.unwrap();
        assert!(deleted.is_none());
    }

    #[tokio::test]
    async fn test_change_id_roundtrip() {
        let store = create_test_store().await;
        let (old_session_id, state) = create_test_record(3600);
        let new_session_id = SessionId::random();

        let record = SessionRecordRef {
            state: Cow::Borrowed(&state),
            ttl: Duration::from_secs(3600),
        };

        // Create session with old ID
        store.create(&old_session_id, record).await.unwrap();

        // Change ID
        store
            .change_id(&old_session_id, &new_session_id)
            .await
            .unwrap();

        // Old ID should not exist
        let old_session = store.load(&old_session_id).await.unwrap();
        assert!(old_session.is_none());

        // New ID should have the data
        let new_session = store.load(&new_session_id).await.unwrap();
        assert!(new_session.is_some());

        let new_record = new_session.unwrap();

        // Verify all data was transferred to new session ID
        for (key, expected_value) in &state {
            assert_eq!(
                new_record.state.get(key).unwrap(),
                expected_value,
                "Mismatch for key after ID change: {}",
                key
            );
        }
    }

    #[tokio::test]
    async fn test_concurrent_operations() {
        let store = create_test_store().await;
        let mut handles = vec![];

        // Create multiple concurrent sessions
        for i in 0..10 {
            let store_clone = store.clone();
            let handle = tokio::spawn(async move {
                let (session_id, state) = create_test_record(3600);
                let mut modified_state = state;
                modified_state.insert(
                    Cow::Borrowed("thread_id"),
                    serde_json::Value::Number(i.into()),
                );

                let record = SessionRecordRef {
                    state: Cow::Borrowed(&modified_state),
                    ttl: Duration::from_secs(3600),
                };

                store_clone.create(&session_id, record).await.unwrap();

                // Verify we can load it back and all data is preserved
                let loaded = store_clone.load(&session_id).await.unwrap().unwrap();

                // Compare against the modified state we created
                for (key, expected_value) in &modified_state {
                    assert_eq!(
                        loaded.state.get(key).unwrap(),
                        expected_value,
                        "Mismatch for key {} in concurrent operation {}",
                        key,
                        i
                    );
                }

                session_id
            });
            handles.push(handle);
        }

        // Wait for all operations to complete
        let mut session_ids = Vec::new();
        for handle in handles {
            session_ids.push(handle.await.unwrap());
        }

        // Verify all sessions exist
        for session_id in session_ids {
            let loaded = store.load(&session_id).await.unwrap();
            assert!(loaded.is_some());
        }
    }

    #[tokio::test]
    async fn test_namespace_isolation() {
        // Connect to redis
        let client = redis::Client::open("redis://localhost:6379").unwrap();
        let conn = tokio::time::timeout(
            Duration::from_secs(2),
            redis::aio::ConnectionManager::new(client),
        )
        .await
        .expect("Failed to connect to Redis within 2 seconds - is Redis running on localhost:6379?")
        .unwrap();

        // Create stores with different namespaces
        let store_a = RedisSessionStore {
            conn: conn.clone(),
            cfg: RedisSessionStoreConfig {
                namespace: Some("a".to_string()),
            },
        };
        let store_b = RedisSessionStore {
            conn: conn.clone(),
            cfg: RedisSessionStoreConfig {
                namespace: Some("b".to_string()),
            },
        };
        let store_c = RedisSessionStore {
            conn: conn.clone(),
            cfg: RedisSessionStoreConfig { namespace: None },
        };

        // Generate and store some session data in each store
        let (session_a, state_a) = create_test_record(3600);
        let record_a = SessionRecordRef {
            state: Cow::Borrowed(&state_a),
            ttl: Duration::from_secs(3600),
        };
        let (session_b, state_b) = create_test_record(3600);
        let record_b = SessionRecordRef {
            state: Cow::Borrowed(&state_b),
            ttl: Duration::from_secs(3600),
        };
        let (session_c, state_c) = create_test_record(3600);
        let record_c = SessionRecordRef {
            state: Cow::Borrowed(&state_c),
            ttl: Duration::from_secs(3600),
        };

        store_a.create(&session_a, record_a).await.unwrap();
        store_b.create(&session_b, record_b).await.unwrap();
        store_c.create(&session_c, record_c).await.unwrap();

        // Each store should only see its own data
        assert!(matches!(store_a.load(&session_a).await.unwrap(), Some(_)));
        assert!(matches!(store_a.load(&session_b).await.unwrap(), None));
        assert!(matches!(store_a.load(&session_c).await.unwrap(), None));

        assert!(matches!(store_b.load(&session_a).await.unwrap(), None));
        assert!(matches!(store_b.load(&session_b).await.unwrap(), Some(_)));
        assert!(matches!(store_b.load(&session_c).await.unwrap(), None));

        assert!(matches!(store_c.load(&session_a).await.unwrap(), None));
        assert!(matches!(store_c.load(&session_b).await.unwrap(), None));
        assert!(matches!(store_c.load(&session_c).await.unwrap(), Some(_)));
    }
}
