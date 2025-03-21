use crate::SessionId;
use errors::{
    ChangeIdError, CreateError, DeleteError, DeleteExpiredError, LoadError, UpdateError,
    UpdateTtlError,
};
use serde_json::Value;
use std::{borrow::Cow, collections::HashMap, num::NonZeroUsize};

/// Where server-side session records are stored.
///
/// It is a thin wrapper
/// [around your chosen storage backend implementation][`SessionStorageBackend`],
/// removing the need to specify the concrete type of the storage backend
/// everywhere in your code.
#[derive(Debug)]
pub struct SessionStore(Box<dyn SessionStorageBackend>);

impl SessionStore {
    /// Creates a new session store using the provided backend.
    pub fn new<Backend>(backend: Backend) -> Self
    where
        Backend: SessionStorageBackend + 'static,
    {
        Self(Box::new(backend))
    }

    /// Creates a new session record in the store using the provided ID.
    pub async fn create(
        &self,
        id: &SessionId,
        record: SessionRecordRef<'_>,
    ) -> Result<(), CreateError> {
        self.0.create(id, record).await
    }

    /// Update the state of an existing session in the store.
    ///
    /// It overwrites the existing record with the provided one.
    pub async fn update(
        &self,
        id: &SessionId,
        record: SessionRecordRef<'_>,
    ) -> Result<(), UpdateError> {
        self.0.update(id, record).await
    }

    /// Update the TTL of an existing session record in the store.
    ///
    /// It leaves the session state unchanged.
    pub async fn update_ttl(
        &self,
        id: &SessionId,
        ttl: std::time::Duration,
    ) -> Result<(), UpdateTtlError> {
        self.0.update_ttl(id, ttl).await
    }

    /// Loads an existing session record from the store using the provided ID.
    ///
    /// If a session with the given ID exists, it is returned. If the session
    /// does not exist or has been invalidated (e.g., expired), `None` is
    /// returned.
    pub async fn load(&self, id: &SessionId) -> Result<Option<SessionRecord>, LoadError> {
        self.0.load(id).await
    }

    /// Deletes a session record from the store using the provided ID.
    ///
    /// If the session exists, it is removed from the store.
    pub async fn delete(&self, id: &SessionId) -> Result<(), DeleteError> {
        self.0.delete(id).await
    }

    /// Change the session id associated with an existing session record.
    ///
    /// The server-side state is left unchanged.
    pub async fn change_id(
        &self,
        old_id: &SessionId,
        new_id: &SessionId,
    ) -> Result<(), ChangeIdError> {
        self.0.change_id(old_id, new_id).await
    }

    /// Deletes expired session records from the store.
    pub async fn delete_expired(
        &self,
        batch_size: Option<NonZeroUsize>,
    ) -> Result<usize, DeleteExpiredError> {
        self.0.delete_expired(batch_size).await
    }
}

#[async_trait::async_trait]
/// The interface of a session storage backend.
pub trait SessionStorageBackend: std::fmt::Debug + Send + Sync {
    /// Creates a new session record in the store using the provided ID.
    async fn create(&self, id: &SessionId, record: SessionRecordRef<'_>)
    -> Result<(), CreateError>;

    /// Update the state of an existing session in the store.
    ///
    /// It overwrites the existing record with the provided one.
    async fn update(&self, id: &SessionId, record: SessionRecordRef<'_>)
    -> Result<(), UpdateError>;

    /// Update the TTL of an existing session record in the store.
    ///
    /// It leaves the session state unchanged.
    async fn update_ttl(
        &self,
        id: &SessionId,
        ttl: std::time::Duration,
    ) -> Result<(), UpdateTtlError>;

    /// Loads an existing session record from the store using the provided ID.
    ///
    /// If a session with the given ID exists, it is returned. If the session
    /// does not exist or has been invalidated (e.g., expired), `None` is
    /// returned.
    async fn load(&self, session_id: &SessionId) -> Result<Option<SessionRecord>, LoadError>;

    /// Deletes a session record from the store using the provided ID.
    ///
    /// If the session exists, it is removed from the store.
    async fn delete(&self, session_id: &SessionId) -> Result<(), DeleteError>;

    /// Change the session id associated with an existing session record.
    ///
    /// The server-side state is left unchanged.
    async fn change_id(&self, old_id: &SessionId, new_id: &SessionId) -> Result<(), ChangeIdError>;

    /// Deletes expired session records from the store.
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
    /// # Do I need to call this method?
    ///
    /// It depends on the storage backend you are using. Some backends (e.g. Redis) have
    /// built-in support for expiring keys, so you may not need to call this method at all.
    ///
    /// If you're adding support for a new backend that has built-in support for expiring keys,
    /// you can simply return `Ok(0)` from this method.
    async fn delete_expired(
        &self,
        batch_size: Option<NonZeroUsize>,
    ) -> Result<usize, DeleteExpiredError>;
}

/// A server-side session record that's going to be stored in the
/// chosen storage backend.
#[derive(Debug)]
pub struct SessionRecordRef<'session> {
    /// The set of key-value pairs attached to a session.
    pub state: Cow<'session, HashMap<Cow<'static, str>, Value>>,
    /// The session time-to-live.
    pub ttl: std::time::Duration,
}

impl SessionRecordRef<'_> {
    pub(crate) fn empty(ttl: std::time::Duration) -> Self {
        Self {
            state: Cow::Owned(HashMap::new()),
            ttl,
        }
    }
}

/// A server-side session record that was retrieved from the
/// chosen storage backend.
#[derive(Debug)]
pub struct SessionRecord {
    /// The set of key-value pairs attached to a session.
    pub state: HashMap<Cow<'static, str>, Value>,
    /// The session time-to-live.
    pub ttl: std::time::Duration,
}

/// Errors that can occur when interacting with a session storage backend.
pub mod errors {
    use crate::SessionId;

    #[non_exhaustive]
    #[derive(Debug, thiserror::Error)]
    /// The error returned by [`SessionStorageBackend::create`][super::SessionStorageBackend::create].
    pub enum CreateError {
        /// Failed to serialize the session state.
        #[error("Failed to serialize the session state.")]
        SerializationError(#[from] serde_json::Error),
        #[error(transparent)]
        /// A session with the same ID already exists.
        DuplicateId(#[from] DuplicateIdError),
        /// Something else went wrong when creating a new session record.
        #[error("Something went wrong when creating a new session record.")]
        Other(#[source] anyhow::Error),
    }

    #[non_exhaustive]
    #[derive(Debug, thiserror::Error)]
    /// The error returned by [`SessionStorageBackend::update`][super::SessionStorageBackend::update].
    pub enum UpdateError {
        #[error("Failed to serialize the session state.")]
        /// Failed to serialize the session state.
        SerializationError(#[from] serde_json::Error),
        #[error(transparent)]
        /// There is no session with the given ID.
        UnknownIdError(#[from] UnknownIdError),
        /// Something else went wrong when updating the session record.
        #[error("Something went wrong when updating the session record.")]
        Other(#[source] anyhow::Error),
    }

    #[non_exhaustive]
    #[derive(Debug, thiserror::Error)]
    /// The error returned by [`SessionStorageBackend::update_ttl`][super::SessionStorageBackend::update_ttl].
    pub enum UpdateTtlError {
        #[error(transparent)]
        /// There is no session with the given ID.
        UnknownId(#[from] UnknownIdError),
        /// Something else went wrong when updating the session record.
        #[error("Something went wrong when updating the TTL of the session record.")]
        Other(#[source] anyhow::Error),
    }

    #[non_exhaustive]
    #[derive(Debug, thiserror::Error)]
    /// The error returned by [`SessionStorageBackend::load`][super::SessionStorageBackend::load].
    pub enum LoadError {
        #[error("Failed to deserialize the session state.")]
        /// Failed to deserialize the session state.
        DeserializationError(#[source] anyhow::Error),
        /// Something else went wrong when loading the session record.
        #[error("Something went wrong when loading the session record.")]
        Other(#[source] anyhow::Error),
    }

    #[non_exhaustive]
    #[derive(Debug, thiserror::Error)]
    /// The error returned by [`SessionStorageBackend::delete`][super::SessionStorageBackend::delete].
    pub enum DeleteError {
        #[error(transparent)]
        /// There is no session with the given ID.
        UnknownId(#[from] UnknownIdError),
        /// Something else went wrong when deleting the session record.
        #[error("Something went wrong when deleting the session record.")]
        Other(#[source] anyhow::Error),
    }

    #[non_exhaustive]
    #[derive(Debug, thiserror::Error)]
    /// The error returned by [`SessionStorageBackend::change_id`][super::SessionStorageBackend::change_id].
    pub enum ChangeIdError {
        #[error(transparent)]
        /// There is no session with the given ID.
        UnknownId(#[from] UnknownIdError),
        #[error(transparent)]
        /// There is already a session associated with the new ID>
        DuplicateId(#[from] DuplicateIdError),
        /// Something else went wrong when deleting the session record.
        #[error("Something went wrong when changing the session id for a session record.")]
        Other(#[source] anyhow::Error),
    }

    /// The error returned by [`SessionStorageBackend::delete_expired`][super::SessionStorageBackend::delete_expired].
    #[derive(Debug, thiserror::Error)]
    #[error("Something went wrong when deleting expired sessions")]
    pub struct DeleteExpiredError(#[from] anyhow::Error);

    #[derive(thiserror::Error)]
    #[error("There is no session with the given id")]
    /// There is no session with the given ID.
    pub struct UnknownIdError {
        pub id: SessionId,
    }

    impl std::fmt::Debug for UnknownIdError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("UnknownIdError")
        }
    }

    #[derive(thiserror::Error)]
    #[error("A session with the same ID already exists.")]
    /// A session with the same ID already exists.
    pub struct DuplicateIdError {
        pub id: SessionId,
    }

    impl std::fmt::Debug for DuplicateIdError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("DuplicateIdError")
        }
    }
}
