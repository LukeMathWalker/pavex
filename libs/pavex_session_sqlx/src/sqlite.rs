//! Types related to [`SqliteSessionStore`].

use pavex::methods;
use pavex::time::Timestamp;
use pavex_session::SessionStore;
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
use sqlx::{
    SqlitePool,
    error::DatabaseError,
    sqlite::{SqliteError, SqliteQueryResult},
};
use std::num::NonZeroUsize;

#[derive(Debug, Clone)]
/// A server-side session store using SQLite as its backend.
///
/// # Implementation details
///
/// This store uses `sqlx` to interact with SQLite.
/// All session records are stored in a single table with JSONB for efficient
/// binary JSON storage (requires SQLite 3.45.0+). You can use
/// [`migrate`](Self::migrate) to create the table and index
/// required by the store in the database.
/// Alternatively, you can use [`migration_query`](Self::migration_query)
/// to get the SQL query that creates the table and index in order to run it yourself
/// (e.g. as part of your database migration scripts).
///
/// # JSONB Support
///
/// This implementation uses SQLite's JSONB format for storing session state,
/// which provides better performance (5-10% smaller size, ~50% faster processing)
/// compared to plain text JSON. JSONB is supported in SQLite 3.45.0 and later.
pub struct SqliteSessionStore(sqlx::SqlitePool);

#[methods]
impl From<SqliteSessionStore> for SessionStore {
    #[singleton]
    fn from(value: SqliteSessionStore) -> Self {
        SessionStore::new(value)
    }
}

#[methods]
impl SqliteSessionStore {
    /// Creates a new SQLite session store instance.
    ///
    /// It requires a pool of SQLite connections to interact with the database
    /// where the session records are stored.
    #[singleton]
    pub fn new(pool: SqlitePool) -> Self {
        Self(pool)
    }

    /// Return the query used to create the sessions table and index.
    ///
    /// # Implementation details
    ///
    /// The query is designed to be idempotent, meaning it can be run multiple times
    /// without causing any issues. If the table and index already exist, the query
    /// does nothing.
    ///
    /// # Alternatives
    ///
    /// You can use this method to add the query to your database migration scripts.
    /// Alternatively, you can use [`migrate`](Self::migrate)
    /// to run the query directly on the database.
    pub fn migration_query() -> &'static str {
        "-- Create the sessions table if it doesn't exist
CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    deadline INTEGER NOT NULL,
    state JSONB NOT NULL
);

-- Create the index on the deadline column if it doesn't exist
CREATE INDEX IF NOT EXISTS idx_sessions_deadline ON sessions(deadline);"
    }

    /// Create the sessions table and index in the database.
    ///
    /// This method is idempotent, meaning it can be called multiple times without
    /// causing any issues. If the table and index already exist, this method does nothing.
    ///
    /// If you prefer to run the query yourself, rely on [`migration_query`](Self::migration_query)
    /// to get the SQL that's being executed.
    pub async fn migrate(&self) -> Result<(), sqlx::Error> {
        use sqlx::Executor as _;

        self.0.execute(Self::migration_query()).await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl SessionStorageBackend for SqliteSessionStore {
    /// Creates a new session record in the store using the provided ID.
    #[tracing::instrument(name = "Create server-side session record", level = tracing::Level::INFO, skip_all)]
    async fn create(
        &self,
        id: &SessionId,
        record: SessionRecordRef<'_>,
    ) -> Result<(), CreateError> {
        let deadline = Timestamp::now() + record.ttl;
        let deadline_unix = deadline.as_second();
        let state = serde_json::to_value(record.state)?;
        let query = sqlx::query(
            "INSERT INTO sessions (id, deadline, state) \
            VALUES (?, ?, ?) \
            ON CONFLICT(id) DO UPDATE \
            SET deadline = excluded.deadline, state = excluded.state \
            WHERE sessions.deadline < unixepoch()",
        )
        .bind(id.inner().to_string())
        .bind(deadline_unix)
        .bind(state);

        match query.execute(&self.0).await {
            // All good, we created the session record.
            Ok(_) => Ok(()),
            Err(e) => {
                // Return the specialized error variant if the ID is already in use
                if let Err(e) = as_duplicated_id_error(&e, id) {
                    Err(e.into())
                } else {
                    Err(CreateError::Other(e.into()))
                }
            }
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
        let new_deadline = Timestamp::now() + record.ttl;
        let new_deadline_unix = new_deadline.as_second();
        let new_state = serde_json::to_value(record.state)?;
        let query = sqlx::query(
            "UPDATE sessions \
            SET deadline = ?, state = ? \
            WHERE id = ? AND deadline > unixepoch()",
        )
        .bind(new_deadline_unix)
        .bind(new_state)
        .bind(id.inner().to_string());

        match query.execute(&self.0).await {
            Ok(r) => as_unknown_id_error(&r, id).map_err(Into::into),
            Err(e) => Err(UpdateError::Other(e.into())),
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
        let new_deadline = Timestamp::now() + ttl;
        let new_deadline_unix = new_deadline.as_second();
        let query = sqlx::query(
            "UPDATE sessions \
            SET deadline = ? \
            WHERE id = ? AND deadline > unixepoch()",
        )
        .bind(new_deadline_unix)
        .bind(id.inner().to_string());
        match query.execute(&self.0).await {
            Ok(r) => as_unknown_id_error(&r, id).map_err(Into::into),
            Err(e) => Err(UpdateTtlError::Other(e.into())),
        }
    }

    /// Loads an existing session record from the store using the provided ID.
    ///
    /// If a session with the given ID exists, it is returned. If the session
    /// does not exist or has been invalidated (e.g., expired), `None` is
    /// returned.
    #[tracing::instrument(name = "Load server-side session record", level = tracing::Level::INFO, skip_all)]
    async fn load(&self, session_id: &SessionId) -> Result<Option<SessionRecord>, LoadError> {
        let row = sqlx::query(
            "SELECT deadline, state \
            FROM sessions \
            WHERE id = ? AND deadline > unixepoch()",
        )
        .bind(session_id.inner().to_string())
        .fetch_optional(&self.0)
        .await
        .map_err(|e| LoadError::Other(e.into()))?;
        row.map(|r| {
            use anyhow::Context as _;
            use sqlx::Row as _;

            let deadline_unix: i64 = r
                .try_get(0)
                .context("Failed to deserialize the retrieved session deadline")
                .map_err(LoadError::DeserializationError)?;
            let deadline = Timestamp::from_second(deadline_unix)
                .context("Failed to parse the retrieved session deadline")
                .map_err(LoadError::DeserializationError)?;
            let state: serde_json::Value = r
                .try_get(1)
                .context("Failed to deserialize the retrieved session state")
                .map_err(LoadError::DeserializationError)?;
            let ttl = deadline - Timestamp::now();
            Ok(SessionRecord {
                // This conversion only fails if the duration is negative, which should not happen
                ttl: ttl.try_into().unwrap_or(std::time::Duration::ZERO),
                state: serde_json::from_value(state)
                    .context("Failed to deserialize the retrieved session state")
                    .map_err(LoadError::DeserializationError)?,
            })
        })
        .transpose()
    }

    /// Deletes a session record from the store using the provided ID.
    ///
    /// If the session exists, it is removed from the store.
    #[tracing::instrument(name = "Delete server-side session record", level = tracing::Level::INFO, skip_all)]
    async fn delete(&self, id: &SessionId) -> Result<(), DeleteError> {
        let query = sqlx::query(
            "DELETE FROM sessions \
            WHERE id = ? AND deadline > unixepoch()",
        )
        .bind(id.inner().to_string());
        match query.execute(&self.0).await {
            Ok(r) => as_unknown_id_error(&r, id).map_err(Into::into),
            Err(e) => Err(DeleteError::Other(e.into())),
        }
    }

    /// Change the session id associated with an existing session record.
    ///
    /// The server-side state is left unchanged.
    #[tracing::instrument(name = "Change id for server-side session record", level = tracing::Level::INFO, skip_all)]
    async fn change_id(&self, old_id: &SessionId, new_id: &SessionId) -> Result<(), ChangeIdError> {
        let query = sqlx::query(
            "UPDATE sessions \
            SET id = ? \
            WHERE id = ? AND deadline > unixepoch()",
        )
        .bind(new_id.inner().to_string())
        .bind(old_id.inner().to_string());
        match query.execute(&self.0).await {
            Ok(r) => as_unknown_id_error(&r, old_id).map_err(Into::into),
            Err(e) => {
                if let Err(e) = as_duplicated_id_error(&e, new_id) {
                    Err(e.into())
                } else {
                    Err(ChangeIdError::Other(e.into()))
                }
            }
        }
    }

    /// Delete expired sessions from the database.
    ///
    /// If `batch_size` is provided, the query will delete at most `batch_size` expired sessions.
    /// In either case, if successful, the method returns the number of expired sessions that
    /// have been deleted.
    ///
    /// # When should you delete in batches?
    ///
    /// If there are a lot of expired sessions in the database, deleting them all at once can
    /// cause performance issues. By deleting in batches, you can limit the number of sessions
    /// deleted in a single query, reducing the impact.
    ///
    /// # Example
    ///
    /// Delete expired sessions in batches of 1000:
    ///
    /// ```no_run
    /// use pavex_session::SessionStore;
    /// use pavex_session_sqlx::SqliteSessionStore;
    /// use pavex_tracing::fields::{
    ///     error_details,
    ///     error_message,
    ///     ERROR_DETAILS,
    ///     ERROR_MESSAGE
    /// };
    /// use std::time::Duration;
    ///
    /// # async fn delete_expired_sessions(pool: sqlx::SqlitePool) {
    /// let backend = SqliteSessionStore::new(pool);
    /// let store = SessionStore::new(backend);
    /// let batch_size = Some(1000.try_into().unwrap());
    /// let batch_sleep = Duration::from_secs(60);
    /// loop {
    ///     if let Err(e) = store.delete_expired(batch_size).await {
    ///         tracing::event!(
    ///             tracing::Level::ERROR,
    ///             { ERROR_MESSAGE } = error_message(&e),
    ///             { ERROR_DETAILS } = error_details(&e),
    ///             "Failed to delete a batch of expired sessions",
    ///         );
    ///     }
    ///     tokio::time::sleep(batch_sleep).await;
    /// }
    /// # }
    async fn delete_expired(
        &self,
        batch_size: Option<NonZeroUsize>,
    ) -> Result<usize, DeleteExpiredError> {
        let query = if let Some(batch_size) = batch_size {
            let batch_size: i64 = batch_size.get().try_into().unwrap_or(i64::MAX);
            sqlx::query("DELETE FROM sessions WHERE id IN (SELECT id FROM sessions WHERE deadline < unixepoch() LIMIT ?)")
                .bind(batch_size)
        } else {
            sqlx::query("DELETE FROM sessions WHERE deadline < unixepoch()")
        };
        let r = query.execute(&self.0).await.map_err(|e| {
            let e: anyhow::Error = e.into();
            e
        })?;
        Ok(r.rows_affected().try_into().unwrap_or(usize::MAX))
    }
}

fn as_duplicated_id_error(e: &sqlx::Error, id: &SessionId) -> Result<(), DuplicateIdError> {
    if let Some(e) = e.as_database_error() {
        if let Some(e) = e.try_downcast_ref::<SqliteError>() {
            // Check if the error is due to a duplicate ID
            // SQLite constraint violation error code is "1555" (SQLITE_CONSTRAINT_PRIMARYKEY)
            if e.code() == Some("1555".into()) {
                return Err(DuplicateIdError { id: id.to_owned() });
            }
        }
    }
    Ok(())
}

fn as_unknown_id_error(r: &SqliteQueryResult, id: &SessionId) -> Result<(), UnknownIdError> {
    // Check if the session record was changed
    if r.rows_affected() == 0 {
        return Err(UnknownIdError { id: id.to_owned() });
    }
    // Sanity check
    assert_eq!(
        r.rows_affected(),
        1,
        "More than one session record was affected, even though the session ID is used as primary key. Something is deeply wrong here!"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pavex_session::store::{SessionRecordRef, SessionStorageBackend};
    use std::borrow::Cow;
    use std::collections::HashMap;
    use std::time::Duration;
    use tempfile::tempdir;

    async fn create_test_store() -> SqliteSessionStore {
        let database_url = "sqlite::memory:";
        let pool = sqlx::SqlitePool::connect(database_url).await.unwrap();
        let store = SqliteSessionStore::new(pool);
        store.migrate().await.unwrap();
        store
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
    async fn test_migration_idempotency() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let database_url = format!("sqlite:{}?mode=rwc", db_path.to_string_lossy());

        let pool = sqlx::SqlitePool::connect(&database_url).await.unwrap();
        let store = SqliteSessionStore::new(pool);

        // Run migration multiple times - should not fail
        store.migrate().await.unwrap();
        store.migrate().await.unwrap();
        store.migrate().await.unwrap();

        // Verify table structure exists and is correct
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='sessions'",
        )
        .fetch_one(&store.0)
        .await
        .unwrap();
        assert_eq!(row.0, 1);

        // Verify index exists
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_sessions_deadline'")
            .fetch_one(&store.0)
            .await
            .unwrap();
        assert_eq!(row.0, 1);
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

        // Verify all data is preserved correctly
        assert_eq!(
            loaded_record.state.get("user_id").unwrap(),
            &serde_json::Value::String("test-user-123".to_string())
        );
        assert_eq!(
            loaded_record.state.get("login_time").unwrap(),
            &serde_json::Value::String("2024-01-01T00:00:00Z".to_string())
        );
        assert_eq!(
            loaded_record.state.get("permissions").unwrap(),
            &serde_json::json!(["read", "write"])
        );

        // Verify nested JSONB structure
        let metadata = loaded_record.state.get("metadata").unwrap();
        assert_eq!(metadata.get("ip").unwrap(), "192.168.1.1");
        assert_eq!(metadata.get("user_agent").unwrap(), "test-agent");
        assert_eq!(metadata.get("session_start").unwrap(), 1640995200);

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

        assert_eq!(
            loaded.state.get("updated_field").unwrap(),
            &serde_json::Value::String("new_value".to_string())
        );
        assert_eq!(
            loaded.state.get("user_id").unwrap(),
            &serde_json::Value::String("updated-user-456".to_string())
        );

        // Verify complex nested structure is preserved
        let new_metadata = loaded.state.get("new_metadata").unwrap();
        assert_eq!(new_metadata.get("last_action").unwrap(), "update_session");
        assert_eq!(new_metadata.get("timestamp").unwrap(), 1640995260);

        let deeply_nested = &new_metadata["complex_data"]["nested"]["deeply"];
        assert_eq!(deeply_nested.as_array().unwrap().len(), 4);
        assert_eq!(deeply_nested[0], "nested");
        assert_eq!(deeply_nested[1], "array");
        assert_eq!(deeply_nested[2], 123);
        assert_eq!(deeply_nested[3], true);

        // Verify TTL was updated
        assert!(loaded.ttl.as_secs() > 7150);
        assert!(loaded.ttl.as_secs() <= 7200);
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
        assert_eq!(
            loaded.state.get("user_id").unwrap(),
            &serde_json::Value::String("test-user-123".to_string())
        );
        assert!(loaded.ttl.as_secs() > 7150);
        assert!(loaded.ttl.as_secs() <= 7200);
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
        assert_eq!(
            new_record.state.get("user_id").unwrap(),
            &serde_json::Value::String("test-user-123".to_string())
        );
    }

    #[tokio::test]
    async fn test_delete_expired() {
        let store = create_test_store().await;

        // Create multiple sessions with different TTLs
        for i in 0..5 {
            let (session_id, state) = create_test_record(if i < 3 { 1 } else { 3600 }); // First 3 expire quickly
            let record = SessionRecordRef {
                state: Cow::Borrowed(&state),
                ttl: Duration::from_secs(if i < 3 { 1 } else { 3600 }),
            };
            store.create(&session_id, record).await.unwrap();
        }

        // Wait for some to expire
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Delete expired sessions
        let deleted_count = store.delete_expired(None).await.unwrap();
        assert_eq!(deleted_count, 3);

        // Run again - should delete 0
        let deleted_count_2 = store.delete_expired(None).await.unwrap();
        assert_eq!(deleted_count_2, 0);
    }

    #[tokio::test]
    async fn test_delete_expired_with_batch_size() {
        let store = create_test_store().await;

        // Create 5 sessions that will expire
        for _ in 0..5 {
            let (session_id, state) = create_test_record(1);
            let record = SessionRecordRef {
                state: Cow::Borrowed(&state),
                ttl: Duration::from_secs(1),
            };
            store.create(&session_id, record).await.unwrap();
        }

        // Wait for expiration
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Delete in batches of 2
        let batch_size = std::num::NonZeroUsize::new(2).unwrap();
        let deleted_1 = store.delete_expired(Some(batch_size)).await.unwrap();
        assert_eq!(deleted_1, 2);

        let deleted_2 = store.delete_expired(Some(batch_size)).await.unwrap();
        assert_eq!(deleted_2, 2);

        let deleted_3 = store.delete_expired(Some(batch_size)).await.unwrap();
        assert_eq!(deleted_3, 1);

        let deleted_4 = store.delete_expired(Some(batch_size)).await.unwrap();
        assert_eq!(deleted_4, 0);
    }

    #[tokio::test]
    async fn test_large_jsonb_data() {
        let store = create_test_store().await;
        let session_id = SessionId::random();

        // Create large, complex JSON structure
        let mut state = HashMap::new();
        let large_string = "x".repeat(10000);
        let large_array: Vec<serde_json::Value> = (0..1000)
            .map(|i| {
                serde_json::json!({
                    "index": i,
                    "data": format!("item_{}", i),
                    "metadata": {
                        "nested": true,
                        "value": i * 2
                    }
                })
            })
            .collect();

        state.insert(
            Cow::Borrowed("large_string"),
            serde_json::Value::String(large_string.clone()),
        );
        state.insert(
            Cow::Borrowed("large_array"),
            serde_json::Value::Array(large_array),
        );
        state.insert(
            Cow::Borrowed("complex_object"),
            serde_json::json!({
                "level1": {
                    "level2": {
                        "level3": {
                            "level4": {
                                "data": "deeply nested",
                                "numbers": [1, 2, 3, 4, 5],
                                "boolean": true,
                                "null_value": null
                            }
                        }
                    }
                }
            }),
        );

        let record = SessionRecordRef {
            state: Cow::Borrowed(&state),
            ttl: Duration::from_secs(3600),
        };

        // Create and load large session
        store.create(&session_id, record).await.unwrap();
        let loaded = store.load(&session_id).await.unwrap().unwrap();

        // Verify large string
        assert_eq!(
            loaded.state.get("large_string").unwrap(),
            &serde_json::Value::String(large_string)
        );

        // Verify large array
        let loaded_array = loaded.state.get("large_array").unwrap().as_array().unwrap();
        assert_eq!(loaded_array.len(), 1000);
        assert_eq!(loaded_array[0]["index"], 0);
        assert_eq!(loaded_array[999]["index"], 999);

        // Verify deeply nested structure
        let complex = &loaded.state["complex_object"]["level1"]["level2"]["level3"]["level4"];
        assert_eq!(complex["data"], "deeply nested");
        assert_eq!(complex["boolean"], true);
        assert!(complex["null_value"].is_null());
    }

    #[tokio::test]
    async fn test_unicode_and_special_characters() {
        let store = create_test_store().await;
        let session_id = SessionId::random();

        let mut state = HashMap::new();
        state.insert(
            Cow::Borrowed("unicode"),
            serde_json::Value::String("Hello, 世界! 🌍 Здравствуй мир! 🎉".to_string()),
        );
        state.insert(
            Cow::Borrowed("json_string"),
            serde_json::Value::String(r#"{"nested": "json", "quotes": "\"escaped\""}"#.to_string()),
        );
        state.insert(
            Cow::Borrowed("special_chars"),
            serde_json::Value::String("Line1\nLine2\tTabbed\rCarriage\"Quoted\"".to_string()),
        );
        state.insert(
            Cow::Borrowed("emoji_data"),
            serde_json::json!({
                "reactions": ["👍", "👎", "❤️", "😂", "😮", "🎉"],
                "message": "Unicode test with émojis and àccénts"
            }),
        );

        let record = SessionRecordRef {
            state: Cow::Borrowed(&state),
            ttl: Duration::from_secs(3600),
        };

        store.create(&session_id, record).await.unwrap();
        let loaded = store.load(&session_id).await.unwrap().unwrap();

        // Verify all special characters and unicode are preserved
        assert_eq!(
            loaded.state.get("unicode").unwrap(),
            &serde_json::Value::String("Hello, 世界! 🌍 Здравствуй мир! 🎉".to_string())
        );
        assert_eq!(
            loaded.state.get("json_string").unwrap(),
            &serde_json::Value::String(
                r#"{"nested": "json", "quotes": "\"escaped\""}"#.to_string()
            )
        );
        assert_eq!(
            loaded.state.get("special_chars").unwrap(),
            &serde_json::Value::String("Line1\nLine2\tTabbed\rCarriage\"Quoted\"".to_string())
        );

        let emoji_data = loaded.state.get("emoji_data").unwrap();
        assert_eq!(emoji_data["reactions"].as_array().unwrap().len(), 6);
        assert_eq!(emoji_data["reactions"][0], "👍");
        assert_eq!(
            emoji_data["message"],
            "Unicode test with émojis and àccénts"
        );
    }

    #[tokio::test]
    async fn test_concurrent_operations() {
        let store = create_test_store().await;
        let mut handles = vec![];

        // Create multiple concurrent sessions
        for i in 0..10 {
            let store_clone = SqliteSessionStore::new(store.0.clone());
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

                // Verify we can load it back
                let loaded = store_clone.load(&session_id).await.unwrap().unwrap();
                assert_eq!(
                    loaded.state.get("thread_id").unwrap(),
                    &serde_json::Value::Number(i.into())
                );

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
}
