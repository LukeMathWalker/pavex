use std::{borrow::Cow, collections::HashMap, num::NonZeroUsize, sync::Arc};

use pavex_session::{
    store::{
        errors::{
            ChangeIdError, CreateError, DeleteError, DeleteExpiredError, LoadError, UpdateError,
            UpdateTtlError,
        },
        SessionRecord, SessionRecordRef, SessionStorageBackend,
    },
    IncomingSession, SessionId, SessionStore,
};
use pavex_session_memory_store::InMemorySessionStore;
use tokio::sync::Mutex;

/// An empty in-memory session store.
pub fn store() -> SessionStore {
    let backend = InMemorySessionStore::default();
    SessionStore::new(backend)
}

/// An empty in-memory session store, with a mechanism to inspect
/// what calls were made to it.
pub fn spy_store() -> (SessionStore, CallTracker) {
    let backend = InMemorySessionStore::default();
    let spy_backend = SpyBackend::new(backend);
    let call_tracker = spy_backend.call_tracker();
    (SessionStore::new(spy_backend), call_tracker)
}

/// A helper to set up a pre-existing session.
pub struct SessionFixture {
    pub id: SessionId,
    pub client_state: HashMap<String, serde_json::Value>,
    /// If `None`, no server-side state will be created.
    pub server_state: Option<HashMap<String, serde_json::Value>>,
    /// If `None`, it'll be defaulted to a value that's high enough
    /// to avoid expiration while we run the test suite.
    pub server_ttl: Option<std::time::Duration>,
}

impl Default for SessionFixture {
    fn default() -> Self {
        Self {
            id: SessionId::random(),
            client_state: HashMap::new(),
            server_state: Some(HashMap::new()),
            server_ttl: None,
        }
    }
}

impl SessionFixture {
    /// Perform the required setup operations and return the `IncomingSession`
    /// instance you need to perform your tests.
    pub async fn setup(&self, store: &SessionStore) -> IncomingSession {
        if let Some(server_state) = &self.server_state {
            let ttl = self
                .server_ttl
                .unwrap_or_else(|| std::time::Duration::from_secs(1000));
            store
                .create(
                    &self.id,
                    SessionRecordRef {
                        state: Cow::Owned(server_state.clone()),
                        ttl,
                    },
                )
                .await
                .expect("Failed to create server-side state for session fixture");
        }

        IncomingSession::from_parts(self.id, self.client_state.clone())
    }

    pub fn id(&self) -> uuid::Uuid {
        self.id.inner()
    }
}

/// A wrapper that keeps track of which methods have been called
/// on the underlying session storage backend
#[derive(Debug)]
pub struct SpyBackend<B> {
    backend: B,
    call_tracker: CallTracker,
}

impl<B> SpyBackend<B> {
    pub fn new(backend: B) -> Self {
        Self {
            backend,
            call_tracker: Default::default(),
        }
    }

    pub fn call_tracker(&self) -> CallTracker {
        self.call_tracker.clone()
    }
}

#[derive(Debug, Clone, Default)]
pub struct CallTracker(Arc<Mutex<CallInformation>>);

impl CallTracker {
    pub async fn assert_store_was_untouched(&self) {
        let info = self.0.lock().await;
        assert!(
            info.oplog.is_empty(),
            "Server store was supposed to be untouched, but at least one method has been called on it. Operation log:\n  - {}",
            info.oplog.join("\n  - ")
        )
    }

    pub async fn assert_never_loaded(&self) {
        assert!(
            !self.0.lock().await.has_invoked_load,
            "Server store tried to load the server state"
        )
    }

    pub async fn operation_log(&self) -> Vec<String> {
        self.0.lock().await.oplog.clone()
    }

    async fn push_operation(&self, op: impl Into<String>) {
        self.0.lock().await.oplog.push(op.into());
    }
}

#[derive(Debug, Clone, Default)]
pub struct CallInformation {
    has_invoked_load: bool,
    oplog: Vec<String>,
}

#[async_trait::async_trait(?Send)]
impl<B: SessionStorageBackend> SessionStorageBackend for SpyBackend<B> {
    async fn create(
        &self,
        id: &SessionId,
        record: SessionRecordRef<'_>,
    ) -> Result<(), CreateError> {
        self.call_tracker
            .push_operation(format!("create {}", id.inner()))
            .await;
        self.backend.create(id, record).await
    }

    /// Update the state of an existing session in the store.
    ///
    /// It overwrites the existing record with the provided one.
    async fn update(
        &self,
        id: &SessionId,
        record: SessionRecordRef<'_>,
    ) -> Result<(), UpdateError> {
        self.call_tracker
            .push_operation(format!("update {}", id.inner()))
            .await;
        self.backend.update(id, record).await
    }

    async fn update_ttl(
        &self,
        id: &SessionId,
        ttl: std::time::Duration,
    ) -> Result<(), UpdateTtlError> {
        self.call_tracker
            .push_operation(format!("update-ttl {}", id.inner()))
            .await;
        self.backend.update_ttl(id, ttl).await
    }

    async fn load(&self, session_id: &SessionId) -> Result<Option<SessionRecord>, LoadError> {
        self.call_tracker
            .push_operation(format!("load {}", session_id.inner()))
            .await;
        self.call_tracker.0.lock().await.has_invoked_load = true;
        self.backend.load(session_id).await
    }

    async fn delete(&self, session_id: &SessionId) -> Result<(), DeleteError> {
        self.call_tracker
            .push_operation(format!("delete {}", session_id.inner()))
            .await;
        self.backend.delete(session_id).await
    }

    async fn change_id(&self, old_id: &SessionId, new_id: &SessionId) -> Result<(), ChangeIdError> {
        self.call_tracker
            .push_operation(format!("change {} {}", old_id.inner(), new_id.inner()))
            .await;
        self.backend.change_id(old_id, new_id).await
    }

    async fn delete_expired(
        &self,
        batch_size: Option<NonZeroUsize>,
    ) -> Result<usize, DeleteExpiredError> {
        let batch_size_fmt = batch_size.map(|b| format!(" {b}")).unwrap_or_default();
        self.call_tracker
            .push_operation(format!("delete-expired{batch_size_fmt}"))
            .await;
        self.backend.delete_expired(batch_size).await
    }
}
