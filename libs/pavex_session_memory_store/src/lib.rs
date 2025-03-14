//! An in-memory session store for `pavex_session`, geared towards testing and local development.
use pavex::time::Timestamp;
use std::{borrow::Cow, collections::HashMap, num::NonZeroUsize, sync::Arc, time::Duration};
use tokio::sync::{Mutex, MutexGuard};

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

pub use kit::{InMemorySessionKit, RegisteredInMemorySessionKit};

#[derive(Clone)]
/// An in-memory session store.
///
/// # Limitations
///
/// This store won't persist data between server restarts.
/// It also won't synchronize data between multiple server instances.
/// It is primarily intended for testing and local development.
pub struct InMemorySessionStore(Arc<Mutex<HashMap<SessionId, StoreRecord>>>);

impl std::fmt::Debug for InMemorySessionStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InMemorySessionStore")
            .finish_non_exhaustive()
    }
}

#[doc(hidden)]
// Here for backwards compatibility.
pub type SessionStoreMemory = InMemorySessionStore;

#[derive(Debug)]
struct StoreRecord {
    state: HashMap<Cow<'static, str>, serde_json::Value>,
    deadline: Timestamp,
}
impl StoreRecord {
    fn is_stale(&self) -> bool {
        self.deadline <= Timestamp::now()
    }
}

impl Default for InMemorySessionStore {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemorySessionStore {
    /// Creates a new (empty) in-memory session store.
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(HashMap::new())))
    }

    fn get_mut_if_fresh<'a, 'b, 'c: 'a>(
        guard: &'a mut MutexGuard<'c, HashMap<SessionId, StoreRecord>>,
        id: &'b SessionId,
    ) -> Result<&'a mut StoreRecord, UnknownIdError> {
        let Some(old_record) = guard.get_mut(id) else {
            return Err(UnknownIdError { id: id.to_owned() });
        };
        if old_record.is_stale() {
            return Err(UnknownIdError { id: id.to_owned() });
        }
        Ok(old_record)
    }

    /// Deletes a session record from the store using the provided ID.
    ///
    /// If the session exists, it is removed from the store.
    fn _delete(
        guard: &mut MutexGuard<'_, HashMap<SessionId, StoreRecord>>,
        id: &SessionId,
    ) -> Result<StoreRecord, UnknownIdError> {
        let Some(old_record) = guard.remove(id) else {
            return Err(UnknownIdError { id: id.to_owned() });
        };
        if old_record.is_stale() {
            return Err(UnknownIdError { id: id.to_owned() });
        }
        Ok(old_record)
    }
}

#[async_trait::async_trait]
impl SessionStorageBackend for InMemorySessionStore {
    /// Creates a new session record in the store using the provided ID.
    #[tracing::instrument(name = "Create server-side session record", level = tracing::Level::TRACE, skip_all)]
    async fn create(
        &self,
        id: &SessionId,
        record: SessionRecordRef<'_>,
    ) -> Result<(), CreateError> {
        let mut guard = self.0.lock().await;
        if Self::get_mut_if_fresh(&mut guard, id).is_ok() {
            return Err(CreateError::DuplicateId(DuplicateIdError { id: *id }));
        }

        guard.insert(
            *id,
            StoreRecord {
                state: record.state.into_owned(),
                deadline: Timestamp::now() + record.ttl,
            },
        );
        Ok(())
    }

    /// Update the state of an existing session in the store.
    ///
    /// It overwrites the existing record with the provided one.
    #[tracing::instrument(name = "Update server-side session record", level = tracing::Level::TRACE, skip_all)]
    async fn update(
        &self,
        id: &SessionId,
        record: SessionRecordRef<'_>,
    ) -> Result<(), UpdateError> {
        let mut guard = self.0.lock().await;
        let old_record = Self::get_mut_if_fresh(&mut guard, id)?;
        *old_record = StoreRecord {
            state: record.state.into_owned(),
            deadline: Timestamp::now() + record.ttl,
        };
        Ok(())
    }

    /// Update the TTL of an existing session record in the store.
    ///
    /// It leaves the session state unchanged.
    #[tracing::instrument(name = "Update TTL for server-side session record", level = tracing::Level::TRACE, skip_all)]
    async fn update_ttl(
        &self,
        id: &SessionId,
        ttl: std::time::Duration,
    ) -> Result<(), UpdateTtlError> {
        let mut guard = self.0.lock().await;
        let old_record = Self::get_mut_if_fresh(&mut guard, id)?;
        old_record.deadline = Timestamp::now() + ttl;
        Ok(())
    }

    /// Loads an existing session record from the store using the provided ID.
    ///
    /// If a session with the given ID exists, it is returned. If the session
    /// does not exist or has been invalidated (e.g., expired), `None` is
    /// returned.
    #[tracing::instrument(name = "Load server-side session record", level = tracing::Level::TRACE, skip_all)]
    async fn load(&self, session_id: &SessionId) -> Result<Option<SessionRecord>, LoadError> {
        let mut guard = self.0.lock().await;
        let outcome = match Self::get_mut_if_fresh(&mut guard, session_id) {
            Ok(old_record) => Some(SessionRecord {
                state: old_record.state.clone(),
                ttl: (old_record.deadline - Timestamp::now())
                    .try_into()
                    .unwrap_or(Duration::from_millis(0)),
            }),
            Err(_) => None,
        };
        Ok(outcome)
    }

    /// Deletes a session record from the store using the provided ID.
    ///
    /// If the session exists, it is removed from the store.
    #[tracing::instrument(name = "Delete server-side session record", level = tracing::Level::TRACE, skip_all)]
    async fn delete(&self, id: &SessionId) -> Result<(), DeleteError> {
        let mut guard = self.0.lock().await;
        Self::_delete(&mut guard, id)?;
        Ok(())
    }

    /// Change the session id associated with an existing session record.
    ///
    /// The server-side state is left unchanged.
    #[tracing::instrument(name = "Change id for server-side session record", level = tracing::Level::TRACE, skip_all)]
    async fn change_id(&self, old_id: &SessionId, new_id: &SessionId) -> Result<(), ChangeIdError> {
        let mut guard = self.0.lock().await;
        if Self::get_mut_if_fresh(&mut guard, new_id).is_ok() {
            return Err(DuplicateIdError {
                id: new_id.to_owned(),
            }
            .into());
        }
        let record = Self::_delete(&mut guard, old_id)?;
        guard.insert(*new_id, record);
        Ok(())
    }

    /// Delete all expired records from the store.
    #[tracing::instrument(name = "Delete expired records", level = tracing::Level::TRACE, skip_all)]
    async fn delete_expired(
        &self,
        batch_size: Option<NonZeroUsize>,
    ) -> Result<usize, DeleteExpiredError> {
        let mut guard = self.0.lock().await;
        let now = Timestamp::now();
        let mut stale_ids = Vec::new();
        for (id, record) in guard.iter() {
            if record.deadline <= now {
                stale_ids.push(*id);
                if let Some(batch_size) = batch_size {
                    if stale_ids.len() >= batch_size.get() {
                        break;
                    }
                }
            }
        }
        let num_deleted = stale_ids.len();
        for id in stale_ids {
            guard.remove(&id);
        }
        Ok(num_deleted)
    }
}

mod kit {
    use pavex::{
        blueprint::{
            Blueprint, config::ConfigType, constructor::Constructor, linter::Lint,
            middleware::PostProcessingMiddleware,
        },
        f,
    };

    #[derive(Clone, Debug)]
    #[non_exhaustive]
    /// Components required to work with sessions using an in-memory store
    /// as the storage backend.
    ///
    /// # Guide
    ///
    /// Check out the [session installation](https://pavex.dev/guide/sessions/installation/)
    /// section of Pavex's guide for a thorough introduction to sessions and how to
    /// customize them.
    ///
    /// # Example
    ///
    /// ```rust
    /// use pavex::blueprint::Blueprint;
    /// use pavex::cookie::CookieKit;
    /// use pavex_session_memory_store::InMemorySessionKit;
    ///
    /// let mut bp = Blueprint::new();
    /// InMemorySessionKit::new().register(&mut bp);
    /// // Sessions are built on top of cookies,
    /// // so you need to set those up too.
    /// // Order is important here!
    /// CookieKit::new().register(&mut bp);
    /// ```
    pub struct InMemorySessionKit {
        /// The constructor for [`Session`].
        ///
        /// By default, it uses [`Session::new`].
        ///
        /// [`Session`]: https://pavex.dev/docs/api_reference/pavex_session/struct.Session.html
        /// [`Session::new`]: https://pavex.dev/docs/api_reference/pavex_session/struct.Session.html#method.new
        pub session: Option<Constructor>,
        /// The constructor for [`IncomingSession`].
        ///
        /// By default, it uses [`IncomingSession::extract`].
        ///
        /// [`IncomingSession`]: https://pavex.dev/docs/api_reference/pavex_session/struct.IncomingSession.html
        /// [`IncomingSession::extract`]: https://pavex.dev/docs/api_reference/pavex_session/struct.IncomingSession.html#method.extract
        pub incoming_session: Option<Constructor>,
        /// Register [`SessionConfig`] as a configuration type.
        ///
        /// By default, it uses `session` as configuration key.
        ///
        /// [`SessionConfig`]: https://pavex.dev/docs/api_reference/pavex_session/struct.SessionConfig.html
        pub session_config: Option<ConfigType>,
        /// The constructor for [`InMemorySessionStore`].
        ///
        /// By default, it uses [`InMemorySessionStore::new`].
        ///
        /// [`InMemorySessionStore`]: crate::InMemorySessionStore
        /// [`InMemorySessionStore::new`]: crate::InMemorySessionStore::new
        pub in_memory_session_store: Option<Constructor>,
        /// The constructor for [`SessionStore`].
        ///
        /// By default, it uses [`SessionStore::new`] with [`InMemorySessionStore`]
        /// as its underlying storage backend.
        ///
        /// [`SessionStore`]: https://pavex.dev/docs/api_reference/pavex_session/struct.SessionStore.html
        /// [`SessionStore::new`]: https://pavex.dev/docs/api_reference/pavex_session/struct.SessionStore.html#method.new
        /// [`InMemorySessionStore`]: crate::InMemorySessionStore
        pub session_store: Option<Constructor>,
        /// A post-processing middleware to sync the session state with the session store
        /// and inject the session cookie into the outgoing response via the `Set-Cookie` header.
        ///
        /// By default, it's set to [`finalize_session`].
        /// The error is handled by [`FinalizeError::into_response`].
        ///
        /// [`FinalizeError::into_response`]: https://pavex.dev/docs/api_reference/pavex_session/errors/enum.FinalizeError.html#method.into_response
        /// [`finalize_session`]: https://pavex.dev/docs/api_reference/pavex_session/fn.finalize_session.html
        pub session_finalizer: Option<PostProcessingMiddleware>,
    }

    impl Default for InMemorySessionKit {
        fn default() -> Self {
            Self::new()
        }
    }

    impl InMemorySessionKit {
        /// Create a new [`InMemorySessionKit`] with all the bundled constructors and middlewares.
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
                in_memory_session_store: Some(
                    Constructor::singleton(f!(crate::InMemorySessionStore::new))
                        .ignore(Lint::Unused),
                ),
                session_store: Some(
                    Constructor::singleton(f!(pavex_session::SessionStore::new::<
                        crate::InMemorySessionStore,
                    >))
                    .ignore(Lint::Unused),
                ),
            }
        }

        #[doc(hidden)]
        #[deprecated(note = "This call is no longer necessary. \
            The session configuration will automatically use its default values if left unspecified.")]
        pub fn with_default_config(self) -> Self {
            self
        }

        /// Register all the bundled constructors and middlewares with a [`Blueprint`].
        ///
        /// If a component is set to `None` it will not be registered.
        pub fn register(self, bp: &mut Blueprint) -> RegisteredInMemorySessionKit {
            let mut kit = pavex_session::SessionKit::new();
            kit.session = self.session;
            kit.incoming_session = self.incoming_session;
            kit.session_config = self.session_config;
            kit.session_finalizer = self.session_finalizer;
            kit.register(bp);
            if let Some(in_memory_session_store) = self.in_memory_session_store {
                in_memory_session_store.register(bp);
            }
            if let Some(session_store) = self.session_store {
                session_store.register(bp);
            }

            RegisteredInMemorySessionKit {}
        }
    }

    #[derive(Clone, Debug)]
    #[non_exhaustive]
    /// The type returned by [`InMemorySessionKit::register`].
    pub struct RegisteredInMemorySessionKit {}
}
