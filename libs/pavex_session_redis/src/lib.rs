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
