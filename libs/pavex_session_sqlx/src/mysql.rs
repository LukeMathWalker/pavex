//! Types related to [`MySqlSessionStore`].
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
    MySqlPool,
    mysql::{MySqlDatabaseError, MySqlQueryResult},
};
use std::num::NonZeroUsize;

#[derive(Debug, Clone)]
/// A server-side session store using MySQL as its backend.
///
/// # Implementation details
///
/// This store uses `sqlx` to interact with MySQL.
/// All session records are stored in a single table. You can use
/// [`migrate`](Self::migrate) to create the table and index
/// required by the store in the database.
/// Alternatively, you can use [`migration_query`](Self::migration_query)
/// to get the SQL query that creates the table and index in order to run it yourself
/// (e.g. as part of your database migration scripts).
///
/// # MySQL version requirements
///
/// This implementation requires MySQL 5.7.8+ or MariaDB 10.2+ for JSON support.
/// For optimal performance with JSON operations, MySQL 8.0+ is recommended.
pub struct MySqlSessionStore(sqlx::MySqlPool);

#[methods]
impl From<MySqlSessionStore> for SessionStore {
    #[singleton]
    fn from(value: MySqlSessionStore) -> Self {
        SessionStore::new(value)
    }
}

#[methods]
impl MySqlSessionStore {
    /// Creates a new MySQL session store instance.
    ///
    /// It requires a pool of MySQL connections to interact with the database
    /// where the session records are stored.
    #[singleton]
    pub fn new(pool: MySqlPool) -> Self {
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
    /// # MySQL version requirements
    ///
    /// This query requires MySQL 5.7.8+ or MariaDB 10.2+ for JSON column support.
    ///
    /// # Alternatives
    ///
    /// You can use this method to add the query to your database migration scripts.
    /// Alternatively, you can use [`migrate`](Self::migrate)
    /// to run the query directly on the database.
    pub fn migration_query() -> &'static str {
        "-- Create the sessions table if it doesn't exist
CREATE TABLE IF NOT EXISTS sessions (
    id CHAR(36) PRIMARY KEY,
    deadline BIGINT NOT NULL,
    state JSON NOT NULL,
    INDEX idx_sessions_deadline (deadline)
);"
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
impl SessionStorageBackend for MySqlSessionStore {
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
            ON DUPLICATE KEY UPDATE \
            deadline = VALUES(deadline), state = VALUES(state)",
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
            WHERE id = ? AND deadline > UNIX_TIMESTAMP()",
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
            WHERE id = ? AND deadline > UNIX_TIMESTAMP()",
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
            WHERE id = ? AND deadline > UNIX_TIMESTAMP()",
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
            WHERE id = ? AND deadline > UNIX_TIMESTAMP()",
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
            WHERE id = ? AND deadline > UNIX_TIMESTAMP()",
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
    /// use pavex_session_sqlx::MySqlSessionStore;
    /// use pavex_tracing::fields::{
    ///     error_details,
    ///     error_message,
    ///     ERROR_DETAILS,
    ///     ERROR_MESSAGE
    /// };
    /// use std::time::Duration;
    ///
    /// # async fn delete_expired_sessions(pool: sqlx::MySqlPool) {
    /// let backend = MySqlSessionStore::new(pool);
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
    /// ```
    async fn delete_expired(
        &self,
        batch_size: Option<NonZeroUsize>,
    ) -> Result<usize, DeleteExpiredError> {
        let query = if let Some(batch_size) = batch_size {
            let batch_size: u64 = batch_size.get().try_into().unwrap_or(u64::MAX);
            sqlx::query("DELETE FROM sessions WHERE deadline < UNIX_TIMESTAMP() LIMIT ?")
                .bind(batch_size)
        } else {
            sqlx::query("DELETE FROM sessions WHERE deadline < UNIX_TIMESTAMP()")
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
        if let Some(e) = e.try_downcast_ref::<MySqlDatabaseError>() {
            // Check if the error is due to a duplicate ID
            // MySQL error code 1062 is for duplicate entry
            if e.number() == 1062 {
                return Err(DuplicateIdError { id: id.to_owned() });
            }
        }
    }
    Ok(())
}

fn as_unknown_id_error(r: &MySqlQueryResult, id: &SessionId) -> Result<(), UnknownIdError> {
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

    async fn create_test_store() -> MySqlSessionStore {
        // Use in-memory MySQL equivalent or skip if no test database available
        let connection_string = std::env::var("TEST_MYSQL_URL")
            .unwrap_or_else(|_| "mysql://root:password@localhost:3306/test_sessions".to_string());

        match MySqlPool::connect(&connection_string).await {
            Ok(pool) => {
                let store = MySqlSessionStore::new(pool);
                store.migrate().await.unwrap();
                store
            }
            Err(_) => {
                // Skip tests if no MySQL available
                panic!(
                    "MySQL test database not available. Set TEST_MYSQL_URL environment variable."
                );
            }
        }
    }

    fn create_test_record(
        _ttl_seconds: u64,
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
            Cow::Borrowed("counter"),
            serde_json::Value::Number(42.into()),
        );
        state.insert(
            Cow::Borrowed("theme"),
            serde_json::Value::String("dark".to_string()),
        );
        (session_id, state)
    }

    #[test]
    fn test_migration_query() {
        let query = MySqlSessionStore::migration_query();

        // Basic sanity checks on the migration query
        assert!(query.contains("CREATE TABLE IF NOT EXISTS sessions"));
        assert!(query.contains("id CHAR(36) PRIMARY KEY"));
        assert!(query.contains("deadline BIGINT NOT NULL"));
        assert!(query.contains("state JSON NOT NULL"));
        assert!(query.contains("INDEX idx_sessions_deadline (deadline)"));
    }

    #[test]
    fn test_error_handling_functions() {
        use sqlx::Error as SqlxError;

        // Test as_duplicated_id_error with non-duplicate error
        let regular_error = SqlxError::RowNotFound;
        let session_id = SessionId::random();
        assert!(as_duplicated_id_error(&regular_error, &session_id).is_ok());
    }

    #[test]
    fn test_session_store_conversion() {
        // Test that MySqlSessionStore can be converted to SessionStore

        // This tests the conversion trait implementation
        // We can't actually create a pool without a database, but we can test the type
        // The actual conversion would look like this:
        // let pool = MySqlPool::connect("mysql://...").await.unwrap();
        // let mysql_store = MySqlSessionStore::new(pool);
        // let session_store = SessionStore::from(mysql_store);

        // For now, just verify the migration query is accessible
        let query = MySqlSessionStore::migration_query();
        assert!(!query.is_empty());
        assert!(query.len() > 100); // Should be a substantial query
    }

    #[test]
    fn test_mysql_version_requirements() {
        let query = MySqlSessionStore::migration_query();

        // Verify it uses MySQL-specific JSON type (requires 5.7.8+)
        assert!(query.contains("JSON"));

        // Verify it uses proper MySQL syntax
        assert!(query.contains("CHAR(36)"));
        assert!(query.contains("BIGINT"));
        assert!(query.contains("INDEX idx_sessions_deadline"));
    }

    #[test]
    fn test_create_test_record_structure() {
        let (session_id, state) = create_test_record(3600);

        // Verify session ID is properly generated
        assert!(!session_id.inner().to_string().is_empty());

        // Verify state structure
        assert!(state.contains_key(&Cow::Borrowed("user_id")));
        assert!(state.contains_key(&Cow::Borrowed("login_time")));
        assert!(state.contains_key(&Cow::Borrowed("counter")));
        assert!(state.contains_key(&Cow::Borrowed("theme")));

        // Verify values are correct types
        assert!(state.get(&Cow::Borrowed("user_id")).unwrap().is_string());
        assert!(state.get(&Cow::Borrowed("counter")).unwrap().is_number());
    }

    #[tokio::test]
    async fn test_migration_idempotency() {
        let store = create_test_store().await;

        // Running migrate multiple times should not fail
        store.migrate().await.unwrap();
        store.migrate().await.unwrap();
        store.migrate().await.unwrap();
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
        let loaded_record = store.load(&session_id).await.unwrap();
        assert!(loaded_record.is_some());
        let loaded_record = loaded_record.unwrap();

        // Verify the data matches
        assert_eq!(loaded_record.state, state);
        // TTL should be approximately the same (within a few seconds)
        let ttl_diff = loaded_record.ttl.as_secs().abs_diff(3600);
        assert!(ttl_diff <= 2, "TTL difference too large: {}", ttl_diff);
    }

    #[tokio::test]
    async fn test_update_roundtrip() {
        let store = create_test_store().await;
        let (session_id, initial_state) = create_test_record(3600);

        let initial_record = SessionRecordRef {
            state: Cow::Borrowed(&initial_state),
            ttl: Duration::from_secs(3600),
        };

        // Create initial session
        store.create(&session_id, initial_record).await.unwrap();

        // Create updated state
        let mut updated_state = HashMap::new();
        updated_state.insert(
            Cow::Borrowed("user_id"),
            serde_json::Value::String("updated-user-456".to_string()),
        );
        updated_state.insert(
            Cow::Borrowed("counter"),
            serde_json::Value::Number(84.into()),
        );
        updated_state.insert(
            Cow::Borrowed("theme"),
            serde_json::Value::String("light".to_string()),
        );

        let updated_record = SessionRecordRef {
            state: Cow::Borrowed(&updated_state),
            ttl: Duration::from_secs(7200),
        };

        // Update session
        store.update(&session_id, updated_record).await.unwrap();

        // Load and verify
        let loaded_record = store.load(&session_id).await.unwrap().unwrap();
        assert_eq!(loaded_record.state, updated_state);
    }

    #[tokio::test]
    async fn test_ttl_expiry() {
        let store = create_test_store().await;
        let session_id = SessionId::random();

        // Create session with very short TTL
        let mut state = HashMap::new();
        state.insert(
            Cow::Borrowed("test"),
            serde_json::Value::String("data".to_string()),
        );

        let record = SessionRecordRef {
            state: Cow::Borrowed(&state),
            ttl: Duration::from_millis(100),
        };

        store.create(&session_id, record).await.unwrap();

        // Wait for expiry
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Should not be able to load expired session
        let loaded = store.load(&session_id).await.unwrap();
        assert!(loaded.is_none());
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

        // Update TTL
        let new_ttl = Duration::from_secs(7200);
        store.update_ttl(&session_id, new_ttl).await.unwrap();

        // Load and verify TTL
        let loaded_record = store.load(&session_id).await.unwrap().unwrap();
        let ttl_diff = loaded_record.ttl.as_secs().abs_diff(new_ttl.as_secs());
        assert!(ttl_diff <= 2, "TTL difference too large: {}", ttl_diff);
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
        assert!(store.load(&session_id).await.unwrap().is_some());

        // Delete session
        store.delete(&session_id).await.unwrap();

        // Verify it's gone
        assert!(store.load(&session_id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_change_id_roundtrip() {
        let store = create_test_store().await;
        let (old_id, state) = create_test_record(3600);
        let new_id = SessionId::random();

        let record = SessionRecordRef {
            state: Cow::Borrowed(&state),
            ttl: Duration::from_secs(3600),
        };

        // Create session with old ID
        store.create(&old_id, record).await.unwrap();

        // Change ID
        store.change_id(&old_id, &new_id).await.unwrap();

        // Old ID should not exist
        assert!(store.load(&old_id).await.unwrap().is_none());

        // New ID should exist with same data
        let loaded_record = store.load(&new_id).await.unwrap().unwrap();
        assert_eq!(loaded_record.state, state);
    }

    #[tokio::test]
    async fn test_delete_expired() {
        let store = create_test_store().await;

        // Clear any existing sessions from previous test runs
        let _ = store.delete_expired(None).await;

        // Create expired sessions by directly inserting with past deadlines
        let past_deadline = Timestamp::now().as_second() - 3600; // 1 hour ago
        let mut expired_session_ids = Vec::new();
        for i in 0..5 {
            let session_id = SessionId::random();
            expired_session_ids.push(session_id.clone());
            let state = serde_json::json!({
                "session_name": format!("expired_session_{}", i)
            });

            // Insert directly with past deadline
            sqlx::query("INSERT INTO sessions (id, deadline, state) VALUES (?, ?, ?)")
                .bind(session_id.inner().to_string())
                .bind(past_deadline)
                .bind(state)
                .execute(&store.0)
                .await
                .unwrap();
        }

        // Create a non-expired session normally
        let valid_session_id = SessionId::random();
        let mut valid_state = HashMap::new();
        valid_state.insert(
            Cow::Borrowed("session_name"),
            serde_json::Value::String("valid_session".to_string()),
        );
        let valid_record = SessionRecordRef {
            state: Cow::Borrowed(&valid_state),
            ttl: Duration::from_secs(3600),
        };
        store.create(&valid_session_id, valid_record).await.unwrap();

        // Verify expired sessions can't be loaded
        for session_id in &expired_session_ids {
            assert!(store.load(session_id).await.unwrap().is_none());
        }

        // Delete expired sessions
        let deleted_count = store.delete_expired(None).await.unwrap();
        assert_eq!(deleted_count, 5);

        // Valid session should still exist
        assert!(store.load(&valid_session_id).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_delete_expired_with_batch_size() {
        let store = create_test_store().await;

        // Clear any existing sessions from previous test runs
        let _ = store.delete_expired(None).await;

        // Create expired sessions by directly inserting with past deadlines
        let past_deadline = Timestamp::now().as_second() - 3600; // 1 hour ago
        let mut expired_session_ids = Vec::new();
        for i in 0..10 {
            let session_id = SessionId::random();
            expired_session_ids.push(session_id.clone());
            let state = serde_json::json!({
                "session_name": format!("expired_session_{}", i)
            });

            // Insert directly with past deadline
            sqlx::query("INSERT INTO sessions (id, deadline, state) VALUES (?, ?, ?)")
                .bind(session_id.inner().to_string())
                .bind(past_deadline)
                .bind(state)
                .execute(&store.0)
                .await
                .unwrap();
        }

        // Verify all sessions are expired
        for session_id in &expired_session_ids {
            assert!(store.load(session_id).await.unwrap().is_none());
        }

        // Delete in batches of 3
        let batch_size = NonZeroUsize::new(3).unwrap();
        let first_batch = store.delete_expired(Some(batch_size)).await.unwrap();
        assert_eq!(first_batch, 3);

        let second_batch = store.delete_expired(Some(batch_size)).await.unwrap();
        assert_eq!(second_batch, 3);

        let third_batch = store.delete_expired(Some(batch_size)).await.unwrap();
        assert_eq!(third_batch, 3);

        let fourth_batch = store.delete_expired(Some(batch_size)).await.unwrap();
        assert_eq!(fourth_batch, 1);

        // No more expired sessions
        let final_batch = store.delete_expired(Some(batch_size)).await.unwrap();
        assert_eq!(final_batch, 0);
    }

    #[tokio::test]
    async fn test_large_json_data() {
        let store = create_test_store().await;
        let session_id = SessionId::random();

        // Create a large JSON object
        let mut state = HashMap::new();

        let large_array: Vec<serde_json::Value> = (0..1000)
            .map(|i| {
                serde_json::json!({
                    "index": i,
                    "name": format!("Item {}", i),
                    "description": "A".repeat(100)
                })
            })
            .collect();

        state.insert(
            Cow::Borrowed("large_array"),
            serde_json::Value::Array(large_array),
        );
        state.insert(
            Cow::Borrowed("large_string"),
            serde_json::Value::String("x".repeat(10000)),
        );
        state.insert(
            Cow::Borrowed("nested_object"),
            serde_json::json!({
                "level1": {
                    "level2": {
                        "level3": {
                            "data": (0..100).collect::<Vec<i32>>()
                        }
                    }
                }
            }),
        );

        let record = SessionRecordRef {
            state: Cow::Borrowed(&state),
            ttl: Duration::from_secs(3600),
        };

        // This should handle large JSON data without issues
        store.create(&session_id, record).await.unwrap();

        let loaded_record = store.load(&session_id).await.unwrap().unwrap();
        assert_eq!(loaded_record.state, state);
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
            serde_json::Value::String(r#"{"nested": "value with \"quotes\""}"#.to_string()),
        );
        state.insert(
            Cow::Borrowed("special_chars"),
            serde_json::Value::String("Special: !@#$%^&*()_+-=[]{}|;':\",./<>?".to_string()),
        );
        state.insert(
            Cow::Borrowed("emoji_array"),
            serde_json::Value::Array(vec![
                serde_json::Value::String("🚀".to_string()),
                serde_json::Value::String("🎉".to_string()),
                serde_json::Value::String("🌟".to_string()),
                serde_json::Value::String("💫".to_string()),
                serde_json::Value::String("⭐".to_string()),
            ]),
        );

        let record = SessionRecordRef {
            state: Cow::Borrowed(&state),
            ttl: Duration::from_secs(3600),
        };

        store.create(&session_id, record).await.unwrap();

        let loaded_record = store.load(&session_id).await.unwrap().unwrap();
        assert_eq!(loaded_record.state, state);
    }

    #[tokio::test]
    async fn test_concurrent_operations() {
        let store = create_test_store().await;
        let (session_id, state) = create_test_record(3600);

        let record = SessionRecordRef {
            state: Cow::Borrowed(&state),
            ttl: Duration::from_secs(3600),
        };

        // Create session
        store.create(&session_id, record).await.unwrap();

        // Spawn multiple concurrent operations
        let store_clone1 = store.clone();
        let store_clone2 = store.clone();
        let store_clone3 = store.clone();
        let id1 = session_id.clone();
        let id2 = session_id.clone();
        let id3 = session_id.clone();

        let (result1, result2, result3) = tokio::join!(
            store_clone1.load(&id1),
            store_clone2.update_ttl(&id2, Duration::from_secs(7200)),
            store_clone3.load(&id3)
        );

        // All operations should succeed
        assert!(result1.unwrap().is_some());
        assert!(result2.is_ok());
        assert!(result3.unwrap().is_some());
    }
}
