//! Types related to [`PostgresSessionStore`].
use jiff_sqlx::ToSqlx;
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
    PgPool,
    postgres::{PgDatabaseError, PgQueryResult},
};
use std::num::NonZeroUsize;

#[derive(Debug, Clone)]
/// A server-side session store using Postgres as its backend.
///
/// # Implementation details
///
/// This store uses `sqlx` to interact with Postgres.
/// All session records are stored in a single table. You can use
/// [`migrate`](Self::migrate) to create the table and index
/// required by the store in the database.
/// Alternatively, you can use [`migration_query`](Self::migration_query)
/// to get the SQL query that creates the table and index in order to run it yourself
/// (e.g. as part of your database migration scripts).
pub struct PostgresSessionStore(sqlx::PgPool);

#[methods]
impl From<PostgresSessionStore> for SessionStore {
    #[singleton]
    fn from(value: PostgresSessionStore) -> Self {
        SessionStore::new(value)
    }
}

#[methods]
impl PostgresSessionStore {
    /// Creates a new Postgres session store instance.
    ///
    /// It requires a pool of Postgres connections to interact with the database
    /// where the session records are stored.
    #[singleton]
    pub fn new(pool: PgPool) -> Self {
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
        "-- Create the sessions table if it doesn’t exist
CREATE TABLE IF NOT EXISTS sessions (
    id UUID PRIMARY KEY,
    deadline TIMESTAMPTZ NOT NULL,
    state JSONB NOT NULL
);

-- Create the index on the deadline column if it doesn’t exist
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_indexes
        WHERE schemaname = current_schema()
            AND tablename = 'sessions'
            AND indexname = 'idx_sessions_deadline'
    ) THEN
        CREATE INDEX idx_sessions_deadline ON sessions(deadline);
    END IF;
END $$;"
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
impl SessionStorageBackend for PostgresSessionStore {
    /// Creates a new session record in the store using the provided ID.
    #[tracing::instrument(name = "Create server-side session record", level = tracing::Level::INFO, skip_all)]
    async fn create(
        &self,
        id: &SessionId,
        record: SessionRecordRef<'_>,
    ) -> Result<(), CreateError> {
        let deadline = Timestamp::now() + record.ttl;
        let state = serde_json::to_value(record.state)?;
        let query = sqlx::query(
            "INSERT INTO sessions (id, deadline, state) \
            VALUES ($1, $2, $3) \
            ON CONFLICT (id) DO UPDATE \
            SET deadline = EXCLUDED.deadline, state = EXCLUDED.state \
            WHERE sessions.deadline < (now() AT TIME ZONE 'UTC')",
        )
        .bind(id.inner())
        .bind(deadline.to_sqlx())
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
        let new_state = serde_json::to_value(record.state)?;
        let query = sqlx::query(
            "UPDATE sessions \
            SET deadline = $1, state = $2 \
            WHERE id = $3 AND deadline > (now() AT TIME ZONE 'UTC')",
        )
        .bind(new_deadline.to_sqlx())
        .bind(new_state)
        .bind(id.inner());

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
        let query = sqlx::query(
            "UPDATE sessions \
            SET deadline = $1 \
            WHERE id = $2 AND deadline > (now() AT TIME ZONE 'UTC')",
        )
        .bind(new_deadline.to_sqlx())
        .bind(id.inner());
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
            WHERE id = $1 AND deadline > (now() AT TIME ZONE 'UTC')",
        )
        .bind(session_id.inner())
        .fetch_optional(&self.0)
        .await
        .map_err(|e| LoadError::Other(e.into()))?;
        row.map(|r| {
            use anyhow::Context as _;
            use sqlx::Row as _;

            let deadline = r
                .try_get::<jiff_sqlx::Timestamp, _>(0)
                .context("Failed to deserialize the retrieved session deadline")
                .map_err(LoadError::DeserializationError)?
                .to_jiff();
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
            WHERE id = $1 AND deadline > (now() AT TIME ZONE 'UTC')",
        )
        .bind(id.inner());
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
            SET id = $1 \
            WHERE id = $2 AND deadline > (now() AT TIME ZONE 'UTC')",
        )
        .bind(new_id.inner())
        .bind(old_id.inner());
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
    /// use pavex_session_sqlx::PostgresSessionStore;
    /// use pavex_tracing::fields::{
    ///     error_details,
    ///     error_message,
    ///     ERROR_DETAILS,
    ///     ERROR_MESSAGE
    /// };
    /// use std::time::Duration;
    ///
    /// # async fn delete_expired_sessions(pool: sqlx::PgPool) {
    /// let backend = PostgresSessionStore::new(pool);
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
            sqlx::query("DELETE FROM sessions WHERE deadline < (now() AT TIME ZONE 'UTC') LIMIT $1")
                .bind(batch_size)
        } else {
            sqlx::query("DELETE FROM sessions WHERE deadline < (now() AT TIME ZONE 'UTC')")
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
        if let Some(e) = e.try_downcast_ref::<PgDatabaseError>() {
            // Check if the error is due to a duplicate ID
            // See https://www.postgresql.org/docs/current/errcodes-appendix.html
            // for the list of error codes for Postgres
            if e.code() == "23505" && e.column() == Some("id") {
                return Err(DuplicateIdError { id: id.to_owned() });
            }
        }
    }
    Ok(())
}

fn as_unknown_id_error(r: &PgQueryResult, id: &SessionId) -> Result<(), UnknownIdError> {
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
