#![deny(missing_docs)]
//! A Redis-based session store for [`pavex_session`](https://crates.io/crates/pavex_session),
//! implemented using the [`redis`](https://crates.io/crates/redis) crate.
use anyhow::Context;
use pavex::{config, methods};
use pavex_session::{
    SessionId, SessionStore,
    store::{
        SessionRecord, SessionRecordRef, SessionStorageBackend,
        errors::{
            ChangeIdError, CreateError, DeleteError, DeleteExpiredError, DuplicateIdError,
            LoadError, UnknownIdError, UpdateError, UpdateTtlError,
        },
    },
};
use redis::{AsyncCommands, ExistenceCheck, SetExpiry, SetOptions, Value, aio::ConnectionManager};
use std::num::NonZeroUsize;

#[config(key = "redis_session_store", default_if_missing)]
#[derive(Clone, Debug, Default, serde::Deserialize)]
/// Configuration options for the Redis session store.
pub struct RedisSessionStoreConfig {
    /// Optional namespace prefix for Redis keys. When set, all session keys will be prefixed with this value.
    ///
    /// Namespacing allows multiple applications to share the same Redis instance without interfering with each other.
    ///
    /// # Example
    ///
    /// If `namespace` is set to `myapp` and the session key is `12345`, then
    /// the session state will be stored in Redis using the key `myapp:12345`.
    #[serde(default)]
    pub namespace: Option<String>,
}

#[derive(Clone)]
/// A server-side session store using Redis as its backend.
///
/// # Implementation details
///
/// This store uses the `redis` crate to interact with Redis. All session records are stored as individual
/// Redis keys with TTL set for automatic expiration. If the `namespace` value in [`RedisSessionStoreConfig`]
/// is `Some`, then all session keys are stored prefixed with this string, allowing multiple applications
/// to share the same Redis instance.
pub struct RedisSessionStore {
    connection: ConnectionManager,
    config: RedisSessionStoreConfig,
}

#[methods]
impl From<RedisSessionStore> for SessionStore {
    #[singleton]
    fn from(s: RedisSessionStore) -> Self {
        SessionStore::new(s)
    }
}

impl std::fmt::Debug for RedisSessionStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RedisSessionStore")
            .field("connection", &"<ConnectionManager>")
            .field("config", &self.config)
            .finish()
    }
}

#[methods]
impl RedisSessionStore {
    /// Creates a new Redis session store instance.
    ///
    /// You must provide a connection as well as configuration.
    #[singleton]
    pub fn new(connection: ConnectionManager, config: RedisSessionStoreConfig) -> Self {
        Self { connection, config }
    }
}

impl RedisSessionStore {
    fn redis_key(&self, id: &SessionId) -> String {
        if let Some(namespace) = &self.config.namespace {
            format!("{}:{}", namespace, id.inner())
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
            .connection
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
            .connection
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
        let mut conn = self.connection.clone();
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
                "Unexpected reply from redis: redis should not report successfully setting ttl for non-existent key"
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
        let mut conn = self.connection.clone();
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
            .connection
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
        let mut conn = self.connection.clone();
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
