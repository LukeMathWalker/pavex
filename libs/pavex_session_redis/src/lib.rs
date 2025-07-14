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
        // Atomically check whether a key exists and then rename if it does.
        const LUA_RENAME_IF_EXISTS: &str = r#"
            if redis.call('EXISTS', KEYS[1]) == 1 then
                -- returns 1 on success or 0 if fails because KEYS[2] already exists
                return redis.call('RENAMENX', KEYS[1], KEYS[2])
            else
                return -1
            end
        "#;

        let script = redis::Script::new(LUA_RENAME_IF_EXISTS);
        let mut conn = self.conn.clone();
        let old_key = self.redis_key(old_id);
        let new_key = self.redis_key(new_id);
        let result: i32 = script
            .key(&old_key)
            .key(&new_key)
            .invoke_async(&mut conn)
            .await
            .map_err(|e| ChangeIdError::Other(e.into()))?;

        match result {
            -1 => Err(err_unknown(old_id).into()),  // Key didn't exist
            1 => Ok(()),                            // Successfully renamed
            0 => Err(err_duplicate(new_id).into()), // Key existed but new_key already exists (RENAMENX failed)
            other => Err(ChangeIdError::Other(anyhow::anyhow!(
                "Redis RENAMENX replied {:?}. Expected INT 0 or 1",
                other
            ))),
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

    // Unhappy path tests - Error scenarios

    #[tokio::test]
    async fn test_create_duplicate_id_error() {
        let store = create_test_store().await;
        let (session_id, state) = create_test_record(3600);

        let record = SessionRecordRef {
            state: Cow::Borrowed(&state),
            ttl: Duration::from_secs(3600),
        };

        // Create initial session
        store.create(&session_id, record).await.unwrap();

        // Try to create another session with the same ID but different data
        let (_, different_state) = create_test_record(7200);
        let mut conflicting_state = different_state;
        conflicting_state.insert(
            Cow::Borrowed("conflict_field"),
            serde_json::Value::String("this should conflict".to_string()),
        );

        let conflicting_record = SessionRecordRef {
            state: Cow::Borrowed(&conflicting_state),
            ttl: Duration::from_secs(1), // Short TTL to force conflict
        };

        // Verify the original session exists
        let loaded = store.load(&session_id).await.unwrap();
        assert!(loaded.is_some());

        // Verify that attempt to create a duplicate session returns an error
        match store.create(&session_id, conflicting_record).await {
            Err(CreateError::DuplicateId(_)) =>  (),
            other => panic!("Expected CreateError::DuplicateId, got {:?}", other)
        };

        // Verify the original data is still there (not overwritten)
        let loaded_after = store.load(&session_id).await.unwrap().unwrap();
        for (key, expected_value) in &state {
            assert_eq!(
                loaded_after.state.get(key).unwrap(),
                expected_value,
                "Original data should be preserved when session is not expired"
            );
        }

        // Verify conflicting data was not written
        assert!(loaded_after.state.get("conflict_field").is_none());
    }

    #[tokio::test]
    async fn test_update_unknown_id_error() {
        let store = create_test_store().await;
        let non_existent_id = SessionId::random();
        let (_, state) = create_test_record(3600);

        let record = SessionRecordRef {
            state: Cow::Borrowed(&state),
            ttl: Duration::from_secs(3600),
        };

        // Try to update a session that doesn't exist
        let result = store.update(&non_existent_id, record).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            pavex_session::store::errors::UpdateError::UnknownIdError(err) => {
                assert!(err.id == non_existent_id);
            }
            other => panic!("Expected UnknownId error, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_update_ttl_unknown_id_error() {
        let store = create_test_store().await;
        let non_existent_id = SessionId::random();

        // Try to update TTL for a session that doesn't exist
        let result = store
            .update_ttl(&non_existent_id, Duration::from_secs(7200))
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            pavex_session::store::errors::UpdateTtlError::UnknownId(err) => {
                assert!(err.id == non_existent_id);
            }
            other => panic!("Expected UnknownId error, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_delete_unknown_id_error() {
        let store = create_test_store().await;
        let non_existent_id = SessionId::random();

        // Try to delete a session that doesn't exist
        let result = store.delete(&non_existent_id).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            pavex_session::store::errors::DeleteError::UnknownId(err) => {
                assert!(err.id == non_existent_id);
            }
            other => panic!("Expected UnknownId error, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_change_id_unknown_old_id_error() {
        let store = create_test_store().await;
        let non_existent_old_id = SessionId::random();
        let new_id = SessionId::random();

        // Try to change ID for a session that doesn't exist
        let result = store.change_id(&non_existent_old_id, &new_id).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            pavex_session::store::errors::ChangeIdError::UnknownId(err) => {
                assert!(err.id == non_existent_old_id);
            }
            other => panic!("Expected UnknownId error, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_change_id_duplicate_new_id_error() {
        let store = create_test_store().await;
        let (session_id_1, state_1) = create_test_record(3600);
        let (session_id_2, state_2) = create_test_record(3600);

        // Create two different sessions
        let record_1 = SessionRecordRef {
            state: Cow::Borrowed(&state_1),
            ttl: Duration::from_secs(3600),
        };
        let record_2 = SessionRecordRef {
            state: Cow::Borrowed(&state_2),
            ttl: Duration::from_secs(3600),
        };

        store.create(&session_id_1, record_1).await.unwrap();
        store.create(&session_id_2, record_2).await.unwrap();

        // Try to change session_id_1 to session_id_2 (which already exists)
        let result = store.change_id(&session_id_1, &session_id_2).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            pavex_session::store::errors::ChangeIdError::DuplicateId(err) => {
                assert!(err.id == session_id_2);
            }
            other => panic!("Expected DuplicateId error, got: {:?}", other),
        }

        // Verify both original sessions still exist
        assert!(store.load(&session_id_1).await.unwrap().is_some());
        assert!(store.load(&session_id_2).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_operations_on_expired_session() {
        let store = create_test_store().await;
        let (session_id, state) = create_test_record(1);

        let record = SessionRecordRef {
            state: Cow::Borrowed(&state),
            ttl: Duration::from_secs(1), // Very short TTL
        };

        // Create session with short TTL
        store.create(&session_id, record).await.unwrap();

        // Wait for expiration
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Try to update expired session - should return UnknownId error
        let (_, new_state) = create_test_record(3600);
        let new_record = SessionRecordRef {
            state: Cow::Borrowed(&new_state),
            ttl: Duration::from_secs(3600),
        };

        let update_result = store.update(&session_id, new_record).await;
        assert!(update_result.is_err());
        match update_result.unwrap_err() {
            pavex_session::store::errors::UpdateError::UnknownIdError(err) => {
                assert!(err.id == session_id);
            }
            other => panic!(
                "Expected UnknownId error for expired session update, got: {:?}",
                other
            ),
        }

        // Try to update TTL of expired session - should return UnknownId error
        let update_ttl_result = store
            .update_ttl(&session_id, Duration::from_secs(7200))
            .await;
        assert!(update_ttl_result.is_err());
        match update_ttl_result.unwrap_err() {
            pavex_session::store::errors::UpdateTtlError::UnknownId(err) => {
                assert!(err.id == session_id);
            }
            other => panic!(
                "Expected UnknownId error for expired session TTL update, got: {:?}",
                other
            ),
        }

        // Try to delete expired session - should return UnknownId error
        let delete_result = store.delete(&session_id).await;
        assert!(delete_result.is_err());
        match delete_result.unwrap_err() {
            pavex_session::store::errors::DeleteError::UnknownId(err) => {
                assert!(err.id == session_id);
            }
            other => panic!(
                "Expected UnknownId error for expired session delete, got: {:?}",
                other
            ),
        }

        // Try to change ID of expired session - should return UnknownId error
        let new_id = SessionId::random();
        let change_id_result = store.change_id(&session_id, &new_id).await;
        assert!(change_id_result.is_err());
        match change_id_result.unwrap_err() {
            pavex_session::store::errors::ChangeIdError::UnknownId(err) => {
                assert!(err.id == session_id);
            }
            other => panic!(
                "Expected UnknownId error for expired session ID change, got: {:?}",
                other
            ),
        }
    }

    #[tokio::test]
    async fn test_serialization_error() {
        let store = create_test_store().await;
        let session_id = SessionId::random();

        // Create a problematic state that might cause serialization issues
        let mut state = HashMap::new();

        // JSON serialization should handle this fine, but let's test with some edge cases
        state.insert(Cow::Borrowed("inf_value"), serde_json::json!(f64::INFINITY));

        let record = SessionRecordRef {
            state: Cow::Borrowed(&state),
            ttl: Duration::from_secs(3600),
        };

        // This should succeed because serde_json handles infinity as null in JSON
        let result = store.create(&session_id, record).await;

        // If it fails, it should be a serialization error
        match result {
            Ok(_) => {
                // Verify we can load it back
                let loaded = store.load(&session_id).await.unwrap().unwrap();
                // Infinity becomes null in JSON
                assert!(loaded.state.get("inf_value").unwrap().is_null());
            }
            Err(pavex_session::store::errors::CreateError::SerializationError(_)) => {
                // This is also acceptable - serialization failed as expected
            }
            Err(other) => panic!("Unexpected error type: {:?}", other),
        }
    }
}
