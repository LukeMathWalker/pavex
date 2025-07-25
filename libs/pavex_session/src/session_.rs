use errors::{
    FinalizeError, ServerGetError, ServerInsertError, ServerRemoveError, SyncError,
    ValueDeserializationError, ValueLocation, ValueSerializationError,
};
use pavex::cookie::{RemovalCookie, ResponseCookie};
use pavex::methods;
use pavex::time::SignedDuration;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::borrow::Cow;
use std::cell::OnceCell;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::MutexGuard;

use crate::SessionConfig;
use crate::SessionId;
use crate::SessionStore;
use crate::State;
use crate::config::{
    MissingServerState, ServerStateCreation, SessionCookieKind, TtlExtensionTrigger,
};
use crate::incoming::IncomingSession;
use crate::store::SessionRecordRef;
use crate::store::errors::{ChangeIdError, DeleteError, LoadError};
use crate::wire::WireClientState;

/// The current HTTP session.
///
/// # Implementation notes
///
/// ## Not `Clone`
///
/// The session is a stateful object that holds the client-side and server-side state
/// of the session, tracking all changes to both states. As a result, `Session` does
/// not implement the `Clone` trait.
///
/// ## Not `Send` nor `Sync`
///
/// The session object is designed to be used within the lifetime of the request
/// it refers to.
/// When Pavex receives a new request, it assigns it to a specific worker thread,
/// where all the processing for that request takes place.
///
/// Given the above, we optimized `Session`'s internals for single-thread usage
/// and decided not to implement `Send` and `Sync` for it.
pub struct Session<'store> {
    id: CurrentSessionId,
    /// The server state is loaded lazily, hence the `OnceCell` wrapper.
    server_state: OnceCell<ServerState>,
    client_state: ClientState,
    /// # Internal invariant
    ///
    /// If the session has been invalidated, `server_state` MUST
    /// be set to `Some(ServerState::MarkedForDeletion)`.
    invalidated: InvalidationFlag,
    store: &'store SessionStore,
    config: &'store SessionConfig,
    /// This field is used to prevent `Send` being implemented for `Session`.
    _unsend: PhantomUnsend,
}

impl std::fmt::Debug for Session<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Session")
            .field("id", &"**redacted**")
            .field("server_state", &self.server_state)
            .field("client_state", &self.client_state)
            .field("invalidated", &self.invalidated)
            .field("store", &self.store)
            .field("config", &self.config)
            .finish()
    }
}

/// A thin wrapper around `OnceCell<()>` to represent an invalidation flag.
#[derive(Clone)]
struct InvalidationFlag(OnceCell<()>);

impl std::fmt::Debug for InvalidationFlag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InvalidationFlag")
            .field("is_invalidated", &self.is_invalidated())
            .finish()
    }
}

impl InvalidationFlag {
    /// Create a new invalidation flag, initially set to `false`.
    fn new() -> Self {
        Self(OnceCell::new())
    }

    /// Set the invalidation flag to `true`.
    fn invalidate(&self) {
        // We don't care if it has already been invalidated.
        let _ = self.0.set(());
    }

    fn is_invalidated(&self) -> bool {
        self.0.get().is_some()
    }
}

/// See <https://stackoverflow.com/questions/62713667/how-to-implement-send-or-sync-for-a-type>
type PhantomUnsend = PhantomData<MutexGuard<'static, ()>>;

#[derive(Clone, PartialEq, Eq)]
enum CurrentSessionId {
    Existing(SessionId),
    /// # Internal invariant
    ///
    /// `old` is always different from `new`.
    ToBeRenamed {
        old: SessionId,
        new: SessionId,
    },
    NewlyGenerated(SessionId),
}

impl CurrentSessionId {
    fn new_id(&self) -> SessionId {
        match self {
            Self::Existing(id) => *id,
            Self::ToBeRenamed { new, .. } => *new,
            Self::NewlyGenerated(id) => *id,
        }
    }

    fn old_id(&self) -> Option<SessionId> {
        match self {
            Self::Existing(id) => Some(*id),
            Self::ToBeRenamed { old, .. } => Some(*old),
            Self::NewlyGenerated(..) => None,
        }
    }
}

#[derive(Debug, Clone)]
enum ClientState {
    Unchanged { state: State },
    Updated { state: State },
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ServerState {
    Unchanged {
        state: State,
        ttl: std::time::Duration,
    },
    DoesNotExist,
    MarkedForDeletion,
    Changed {
        state: State,
    },
}

#[methods]
impl<'store> Session<'store> {
    /// Create a new HTTP session.
    ///
    /// It is a continuation of the existing session if there was a valid session cookie
    /// attached to the request.
    /// It is a brand-new session otherwise.
    #[request_scoped]
    pub fn new(
        store: &'store SessionStore,
        config: &'store SessionConfig,
        incoming_session: Option<IncomingSession>,
    ) -> Self {
        let (client_state, previous_session_id) = match incoming_session {
            Some(s) => (s.client_state, Some(s.id)),
            None => (Default::default(), None),
        };
        let (id, server_state) = match previous_session_id {
            Some(id) => (CurrentSessionId::Existing(id), None),
            None => (
                CurrentSessionId::NewlyGenerated(SessionId::random()),
                Some(ServerState::DoesNotExist),
            ),
        };
        Self {
            id,
            server_state: new_cell_with(server_state),
            client_state: ClientState::Unchanged {
                state: client_state,
            },
            invalidated: InvalidationFlag::new(),
            store,
            config,
            _unsend: Default::default(),
        }
    }
}

/// All the operations you can perform on the server-side state of your session.
impl Session<'_> {
    /// Get the value associated with `key` from the server-side state.
    ///
    /// If the value is not found, `None` is returned.\
    /// If the value is found, but it cannot be deserialized into the expected type,
    /// an error is returned.
    ///
    /// If you don't need to deserialize the value, or you'd like to handle the deserialization
    /// yourself, use [`get_raw`][Self::get_raw] instead.
    pub async fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>, ServerGetError> {
        self.get_raw(key)
            .await?
            .map(|value| serde_json::from_value(value.clone()))
            .transpose()
            .map_err(|e| {
                ValueDeserializationError {
                    key: key.to_string().into(),
                    location: ValueLocation::Server,
                    source: e,
                }
                .into()
            })
    }

    /// Insert a value for the given key in the server-side state.
    ///
    /// If the state didn't have an entry for this key, the value is inserted and `None` is returned.\
    /// If the state did have an entry for this key, its value is updated and the old
    /// value is returned in its raw JSON form.
    ///
    /// The provided value is serialized to JSON prior to being stored. If
    /// the serialization fails, an error is returned. If you'd prefer to
    /// take care of the serialization yourself, use [`insert_raw`][Self::insert_raw] instead.
    pub async fn insert<T, Key>(
        &mut self,
        key: Key,
        value: T,
    ) -> Result<Option<Value>, ServerInsertError>
    where
        T: Serialize,
        Key: Into<Cow<'static, str>>,
    {
        let key = key.into();
        let value = match serde_json::to_value(value) {
            Ok(t) => t,
            Err(source) => {
                return Err(ValueSerializationError {
                    key,
                    location: ValueLocation::Server,
                    source,
                }
                .into());
            }
        };
        self.insert_raw(key, value).await.map_err(Into::into)
    }

    /// Remove the value associated with `key` from the server-side state.
    ///
    /// If the key doesn't exist, `None` is returned.
    ///
    /// If the key exists, the removed value is returned, deserialized into the type you specify as `T`.
    /// If the removed value cannot be deserialized, an error is returned.
    ///
    /// If you're not interested in the removed value, or you don't want to deserialize it,
    /// use [`remove_raw`][Self::remove_raw] instead.
    pub async fn remove<T: DeserializeOwned>(
        &mut self,
        key: &str,
    ) -> Result<Option<T>, ServerRemoveError> {
        self.remove_raw(key)
            .await?
            .map(serde_json::from_value)
            .transpose()
            .map_err(|source| ValueDeserializationError {
                key: key.to_string().into(),
                location: ValueLocation::Server,
                source,
            })
            .map_err(ServerRemoveError::DeserializationError)
    }

    /// Returns `true` if there are no values in the server-side state.
    pub async fn is_empty(&self) -> Result<bool, LoadError> {
        use ServerState::*;

        match force_load_ref(self).await? {
            Unchanged { state, .. } | Changed { state } => Ok(state.is_empty()),
            DoesNotExist | MarkedForDeletion => Ok(true),
        }
    }

    /// Get the value associated with `key` from the server-side state.
    pub async fn get_raw<'a>(&'a self, key: &str) -> Result<Option<&'a Value>, LoadError> {
        use ServerState::*;

        match force_load_ref(self).await? {
            Unchanged { state, .. } | Changed { state } => Ok(state.get(key)),
            DoesNotExist => Ok(None),
            MarkedForDeletion => {
                tracing::debug!(session.key = %key, "Tried to access a server-side value on a session marked for deletion.");
                Ok(None)
            }
        }
    }

    /// Insert a value for the given key in the server-side state.
    ///
    /// If the state didn't have an entry for this key, the value is inserted and `None` is returned.\
    /// If the state did have an entry for this key, its value is updated and the old
    /// value is returned in its raw JSON form.
    ///
    /// The provided value must be a JSON value, which will be stored as-is, without any
    /// further manipulation. If you'd prefer to let `pavex_session` handle the serialization,
    /// use [`insert`][Self::insert] instead.
    pub async fn insert_raw<Key>(
        &mut self,
        key: Key,
        value: Value,
    ) -> Result<Option<Value>, LoadError>
    where
        Key: Into<Cow<'static, str>>,
    {
        let mut existing_state;
        let key = key.into();

        use ServerState::*;
        match force_load_mut(self).await? {
            MarkedForDeletion => {
                tracing::debug!(session.key = %key, "Tried to insert a server-side value on a session marked for deletion.");
                return Ok(None);
            }
            Unchanged { state, .. } | Changed { state } => {
                existing_state = std::mem::take(state);
            }
            DoesNotExist => {
                existing_state = HashMap::new();
            }
        };
        let old_value = existing_state.insert(key, value);
        self.server_state = new_cell_with(Some(ServerState::Changed {
            state: existing_state,
        }));
        Ok(old_value)
    }

    /// Remove the value associated with `key` from the server-side state.
    ///
    /// If the key exists, the removed value is returned.\
    /// The value is returned as it was stored in the server-side state, without any deserialization.
    /// If you want to deserialize the value as a specific type, use [`remove`][Self::remove] instead.
    pub async fn remove_raw(&mut self, key: &str) -> Result<Option<Value>, LoadError> {
        use ServerState::*;
        match force_load_mut(self).await? {
            MarkedForDeletion => {
                tracing::debug!(session.key = %key, "Tried to delete a server-side value on a session marked for deletion.");
                Ok(None)
            }
            DoesNotExist => Ok(None),
            Unchanged { state, .. } | Changed { state } => Ok(state.remove(key)),
        }
    }

    /// Delete the session record from the store.
    ///
    /// This doesn't destroy the whole session—you must invoke [`Session::invalidate`]
    /// if that's your goal.
    pub fn delete(&mut self) {
        self.server_state = new_cell_with(Some(ServerState::MarkedForDeletion));
    }

    /// Remove all key-value pairs from the server-side state.
    ///
    /// This doesn't delete the session record from the store—you must invoke
    /// [`Session::delete`][Self::delete] if you want to delete the record altogether.
    ///
    /// This doesn't invalidate the session—you must invoke [`Session::invalidate`]
    /// if you want to delete the session altogether.
    pub async fn clear(&mut self) -> Result<(), LoadError> {
        use ServerState::*;
        match force_load_mut(self).await? {
            MarkedForDeletion | DoesNotExist => {}
            Unchanged { state, .. } => {
                if !state.is_empty() {
                    self.server_state = new_cell_with(Some(ServerState::Changed {
                        state: HashMap::new(),
                    }));
                }
            }
            Changed { state } => {
                state.clear();
            }
        }
        Ok(())
    }

    /// Generate a new session identifier and attach it to this session.
    /// The session state is preserved on both the client-side and the server-side.
    ///
    /// This method is useful for security reasons, as it can help prevent
    /// [session fixation attacks](https://owasp.org/www-community/attacks/Session_fixation).
    pub fn cycle_id(&mut self) {
        let old = match &self.id {
            CurrentSessionId::Existing(id) => Some(*id),
            CurrentSessionId::ToBeRenamed { old, .. } => Some(*old),
            CurrentSessionId::NewlyGenerated(_) => None,
        };

        static MAX_N_ATTEMPTS: usize = 16;

        let mut i = 0;
        let new = loop {
            if i >= MAX_N_ATTEMPTS {
                panic!(
                    "Failed to generate a new session ID that doesn't collide with the pre-existing one, \
                    even though {MAX_N_ATTEMPTS} attempts were carried out. Something seems to be seriously wrong \
                    with the underlying source of randomness."
                )
            }

            let new = SessionId::random();
            if Some(new) != old {
                break new;
            } else {
                i += 1;
            }
        };

        self.id = match old {
            Some(old) => CurrentSessionId::ToBeRenamed { old, new },
            None => CurrentSessionId::NewlyGenerated(new),
        };
    }

    /// Invalidate the session.
    ///
    /// The server-side session state will be marked for deletion.
    /// The client-side cookie will be removed from the client using a removal cookie.
    ///
    /// After calling this method, the session is considered invalid and should not be used anymore.
    /// All further operations on the session will be no-ops.
    pub fn invalidate(&mut self) {
        self.invalidated.invalidate();
        self.server_state = new_cell_with(Some(ServerState::MarkedForDeletion));
    }

    /// Check if the session has been invalidated.
    ///
    /// See [`Session::invalidate`] for more information.
    pub fn is_invalidated(&self) -> bool {
        self.invalidated.is_invalidated()
    }
}

/// Control when the server-side state is synchronized with the store.
impl Session<'_> {
    /// Sync the in-memory representation of the server-side state
    /// with the store.
    ///
    /// In most cases, you don't need to invoke this method manually: it is
    /// done for you by [`finalize_session`][`super::finalize_session`],
    /// the post-processing middleware that attaches the session cookie to
    /// the response returned to the client.
    pub async fn sync(&mut self) -> Result<(), SyncError> {
        let state_config = &self.config.state;
        let fresh_ttl = state_config.ttl;
        let create_if_empty = {
            let has_client_side = self.id.old_id().is_some()
                || matches!(self.client_state, ClientState::Updated { .. });
            has_client_side && state_config.server_state_creation == ServerStateCreation::NeverSkip
        };
        use ServerState::*;
        match self.server_state.get() {
            Some(DoesNotExist) => match self.id {
                CurrentSessionId::NewlyGenerated(id) | CurrentSessionId::Existing(id) => {
                    if create_if_empty {
                        self.store
                            .create(&id, SessionRecordRef::empty(fresh_ttl))
                            .await?;
                    }
                }
                CurrentSessionId::ToBeRenamed { .. } => {
                    // Nothing to do.
                }
            },
            None => {
                match self.id {
                    CurrentSessionId::Existing(_) => {
                        // Nothing to do.
                    }
                    CurrentSessionId::ToBeRenamed { old, new } => {
                        self.store.change_id(&old, &new).await?;
                    }
                    CurrentSessionId::NewlyGenerated(..) => {
                        unreachable!(
                            "A newly generated session cannot have a 'NotLoaded' server state. It must be set to 'DoesNotExist'."
                        )
                    }
                };
            }
            Some(Unchanged {
                state,
                ttl: remaining_ttl,
            }) => {
                match self.id {
                    CurrentSessionId::Existing(old) => {
                        if state_config.extend_ttl == TtlExtensionTrigger::OnStateLoadsAndChanges {
                            let extend = state_config
                                .ttl_extension_threshold
                                .map(|ratio| *remaining_ttl < fresh_ttl.mul_f32(ratio.inner()))
                                .unwrap_or(true);
                            if extend {
                                self.store.update_ttl(&old, fresh_ttl).await?;
                            }
                        }
                    }
                    CurrentSessionId::ToBeRenamed { old, new } => {
                        match self.store.change_id(&old, &new).await {
                            Ok(_) => {}
                            Err(ChangeIdError::UnknownId(_)) => {
                                // The old state is no longer in the store—e.g. it may have
                                // expired while we were processing. Rare, but possible.
                                // We know what the new state needs to be though, so we
                                // can handle this edge case gracefully.
                                let record = SessionRecordRef {
                                    state: Cow::Borrowed(state),
                                    ttl: fresh_ttl,
                                };
                                self.store.create(&new, record).await?;
                            }
                            Err(e) => {
                                return Err(e.into());
                            }
                        }
                    }
                    CurrentSessionId::NewlyGenerated(new) => {
                        if create_if_empty {
                            self.store
                                .create(&new, SessionRecordRef::empty(fresh_ttl))
                                .await?;
                        }

                        // Integrity check.
                        assert!(
                            state.is_empty(),
                            "Server state is not empty on a new session, \
                                    but the state is marked as 'unchanged'. This is a bug in `pavex_session`"
                        );
                    }
                };
            }
            Some(MarkedForDeletion) => match self.id.old_id() {
                Some(id) => {
                    if let Err(e) = self.store.delete(&id).await {
                        match e {
                            // We're good as long as we made sure that no server-side
                            // state is stored against this id, we're good.
                            DeleteError::UnknownId(_) => {}
                            _ => return Err(e.into()),
                        }
                    }
                }
                None => {
                    tracing::trace!(
                        "The server session state was marked for deletion, but there was no session to delete. This is a no-op."
                    )
                }
            },
            Some(Changed { state }) => {
                let record = SessionRecordRef {
                    state: Cow::Borrowed(state),
                    ttl: fresh_ttl,
                };
                match self.id {
                    CurrentSessionId::Existing(id) => {
                        self.store.update(&id, record).await?;
                    }
                    CurrentSessionId::ToBeRenamed { old, new } => {
                        if let Err(e) = self.store.delete(&old).await {
                            match e {
                                DeleteError::UnknownId(_) => {
                                    // The record may have expired between this
                                    // delete operation and the first (successful)
                                    // load we performed at the beginning of this
                                    // request processing task.
                                    // Since we already have the value in memory,
                                    // this is not an issue.
                                }
                                _ => {
                                    return Err(e.into());
                                }
                            }
                        }
                        self.store.create(&new, record).await?;
                    }
                    CurrentSessionId::NewlyGenerated(id) => {
                        self.store.create(&id, record).await?;
                    }
                }
            }
        };

        self.server_state = {
            let old_state = self.server_state.take();
            let new_state = old_state.map(|state| match state {
                Changed { state } => Unchanged {
                    state,
                    ttl: fresh_ttl,
                },
                Unchanged { state, ttl } => Unchanged { state, ttl },
                MarkedForDeletion => {
                    if self.is_invalidated() {
                        MarkedForDeletion
                    } else {
                        DoesNotExist
                    }
                }
                DoesNotExist => {
                    if create_if_empty {
                        Unchanged {
                            state: HashMap::new(),
                            ttl: fresh_ttl,
                        }
                    } else {
                        DoesNotExist
                    }
                }
            });
            new_cell_with(new_state)
        };
        Ok(())
    }

    /// Load the server-side state from the store.
    /// This method does nothing if the server-side state has already been loaded.
    ///
    /// After calling this method, the server-side state will be loaded
    /// and cached in memory, so that subsequent calls to [`get_raw`](#method.get_raw),
    /// [`insert_raw`](#method.insert_raw), and [`remove_raw`](#method.remove_raw)
    /// will operate on the in-memory state.
    pub async fn force_load(&self) -> Result<(), LoadError> {
        force_load(self).await
    }

    /// Sync the current server-side state with the chosen storage backend.
    /// If necessary, it returns a cookie to be attached to the outgoing response
    /// in order to sync the client-side state.
    #[must_use = "The cookie returned by `finalize` must be attached to the outgoing HTTP response. \
        Failing to do so will push the session into an invalid state."]
    pub async fn finalize(&mut self) -> Result<Option<ResponseCookie<'static>>, FinalizeError> {
        self.sync().await?;

        let cookie_config = &self.config.cookie;
        let cookie_name = &cookie_config.name;

        if self.invalidated.is_invalidated() {
            if self.id.old_id().is_none() {
                // This is a new session, so there's nothing on the client-side
                // to be removed.
                return Ok(None);
            }
            let mut cookie = RemovalCookie::new(cookie_name.clone());
            if let Some(domain) = cookie_config.domain.as_deref() {
                cookie = cookie.set_domain(domain.to_owned());
            }
            if let Some(path) = cookie_config.path.as_deref() {
                cookie = cookie.set_path(path.to_owned());
            }
            Ok(Some(cookie.into()))
        } else {
            match &self.client_state {
                ClientState::Updated {
                    state: client_state,
                }
                | ClientState::Unchanged {
                    state: client_state,
                } => {
                    let server_record_exists = match &self.server_state.get() {
                        None => None,
                        Some(ServerState::Unchanged { .. }) => Some(true),
                        Some(ServerState::DoesNotExist) => Some(false),
                        Some(ServerState::MarkedForDeletion)
                        | Some(ServerState::Changed { .. }) => {
                            unreachable!("The server state has just been synchronized.")
                        }
                    };
                    // The session is new, we don't have a server-side record, and the client state is empty.
                    // We don't need to create a session cookie in this case.
                    if client_state.is_empty()
                        && self.id.old_id().is_none()
                        && !server_record_exists.unwrap_or(true)
                    {
                        return Ok(None);
                    }
                    let value = WireClientState {
                        session_id: self.id.new_id(),
                        user_values: Cow::Borrowed(client_state),
                    };
                    let value = serde_json::to_string(&value)?;
                    let mut cookie = ResponseCookie::new(cookie_name.clone(), value);
                    if let Some(domain) = cookie_config.domain.as_deref() {
                        cookie = cookie.set_domain(domain.to_owned());
                    }
                    if let Some(path) = cookie_config.path.as_deref() {
                        cookie = cookie.set_path(path.to_owned());
                    }
                    if let Some(same_site) = cookie_config.same_site {
                        cookie = cookie.set_same_site(same_site);
                    }
                    if cookie_config.secure {
                        cookie = cookie.set_secure(true);
                    }
                    if cookie_config.http_only {
                        cookie = cookie.set_http_only(true);
                    }
                    if cookie_config.kind == SessionCookieKind::Persistent {
                        let max_age: SignedDuration = self
                            .config
                            .state
                            .ttl
                            .try_into()
                            .unwrap_or(SignedDuration::MAX);
                        cookie = cookie.set_max_age(max_age);
                    }
                    Ok(Some(cookie))
                }
            }
        }
    }
}

/// APIs for manipulating the client-side session state.
impl Session<'_> {
    /// Read values from the client-side state attached to this session.
    pub fn client(&self) -> ClientSessionState<'_> {
        ClientSessionState(&self.client_state, &self.invalidated)
    }

    /// Read or mutate the client-side state attached to this session.
    pub fn client_mut(&mut self) -> ClientSessionStateMut<'_> {
        ClientSessionStateMut(&mut self.client_state, &self.invalidated)
    }
}

/// A read-only reference to the client-side state of a session.
pub struct ClientSessionState<'session>(&'session ClientState, &'session InvalidationFlag);

impl<'session> ClientSessionState<'session> {
    /// Get the value associated with `key` from the client-side state.
    ///
    /// If the value is not found, `None` is returned.
    /// If the value is found, but it cannot be deserialized into the expected type, an error is returned.
    pub fn get<T: DeserializeOwned>(
        &self,
        key: &str,
    ) -> Result<Option<T>, ValueDeserializationError> {
        client_get(self.0, self.1, key)
    }

    /// Get the raw JSON value associated with `key` from the client-side state.
    pub fn get_raw(&self, key: &str) -> Option<&'session Value> {
        client_get_raw(self.0, self.1, key)
    }

    /// Returns true if there are no values in the client-side state.
    pub fn is_empty(&self) -> bool {
        client_is_empty(self.0, self.1)
    }
}

/// A mutable reference to the client-side state of a session.
pub struct ClientSessionStateMut<'session>(&'session mut ClientState, &'session InvalidationFlag);

impl ClientSessionStateMut<'_> {
    /// Get the value associated with `key` from the client-side state.
    ///
    /// If the value is not found, `None` is returned.
    /// If the value is found, but it cannot be deserialized into the expected type, an error is returned.
    pub fn get<T: DeserializeOwned>(
        &self,
        key: &str,
    ) -> Result<Option<T>, ValueDeserializationError> {
        client_get(self.0, self.1, key)
    }

    /// Get the raw JSON value associated with `key` from the client-side state.
    pub fn get_raw<'a>(&'a self, key: &str) -> Option<&'a Value> {
        client_get_raw(&*self.0, self.1, key)
    }

    /// Returns true if there are no values in the client-side state.
    pub fn is_empty(&self) -> bool {
        client_is_empty(self.0, self.1)
    }

    /// Insert a value in the client-side state for the given key.
    ///
    /// If the key already exists, the value is updated and the old raw value is returned.
    /// If the value cannot be serialized, an error is returned.
    pub fn insert<T, Key>(
        &mut self,
        key: Key,
        value: T,
    ) -> Result<Option<Value>, ValueSerializationError>
    where
        T: Serialize,
        Key: Into<Cow<'static, str>>,
    {
        let key = key.into();
        let value = match serde_json::to_value(value) {
            Ok(t) => t,
            Err(e) => {
                return Err(ValueSerializationError {
                    key,
                    location: ValueLocation::Client,
                    source: e,
                });
            }
        };
        Ok(self.insert_raw(key, value))
    }

    /// Insert a value in the client-side state for the given key.
    ///
    /// If the key already exists, the value is updated and the old value is returned.
    pub fn insert_raw<Key>(&mut self, key: Key, value: Value) -> Option<Value>
    where
        Key: Into<Cow<'static, str>>,
    {
        if self.1.is_invalidated() {
            tracing::trace!(
                "Attempted to insert a client-side value on a session that's been invalidated."
            );
            return None;
        }
        let key = key.into();
        match &mut self.0 {
            ClientState::Updated { state } => state.insert(key, value),
            ClientState::Unchanged { state } => {
                let value = state.insert(key, value);
                *self.0 = ClientState::Updated {
                    state: std::mem::take(state),
                };
                value
            }
        }
    }

    /// Remove the value associated with `key` from the client-side state.
    ///
    /// If the key exists, the removed value is returned.
    /// If the removed value cannot be serialized, an error is returned.
    pub fn remove<T: DeserializeOwned>(
        &mut self,
        key: &str,
    ) -> Result<Option<T>, ValueDeserializationError> {
        self.remove_raw(key)
            .map(|value| serde_json::from_value(value))
            .transpose()
            .map_err(|source| ValueDeserializationError {
                key: key.to_string().into(),
                location: ValueLocation::Client,
                source,
            })
    }

    /// Remove the value associated with `key` from the client-side state.
    ///
    /// If the key exists, the removed value is returned.
    pub fn remove_raw(&mut self, key: &str) -> Option<Value> {
        if self.1.is_invalidated() {
            return None;
        }
        match &mut self.0 {
            ClientState::Updated { state } => state.remove(key),
            ClientState::Unchanged { state } => {
                let value = state.remove(key)?;
                *self.0 = ClientState::Updated {
                    state: std::mem::take(state),
                };
                Some(value)
            }
        }
    }

    /// Remove all key-value pairs from the client-side state.
    ///
    /// This doesn't invalidate the session—you must invoke [`Session::invalidate`]
    /// if you want to delete the session altogether.
    pub fn clear(&mut self) {
        if self.1.is_invalidated() {
            return;
        }
        match &mut self.0 {
            ClientState::Updated { state } => state.clear(),
            ClientState::Unchanged { state } => {
                if !state.is_empty() {
                    *self.0 = ClientState::Updated {
                        state: HashMap::new(),
                    };
                }
            }
        }
    }
}

/// Get the value associated with `key` from the client-side state.
///
/// If the value is not found, `None` is returned.
/// If the value is found, but it cannot be deserialized into the expected type, an error is returned.
fn client_get<T: DeserializeOwned>(
    state: &ClientState,
    flag: &InvalidationFlag,
    key: &str,
) -> Result<Option<T>, ValueDeserializationError> {
    client_get_raw(state, flag, key)
        .map(|value| serde_json::from_value(value.clone()))
        .transpose()
        .map_err(|source| ValueDeserializationError {
            location: ValueLocation::Client,
            key: key.to_string().into(),
            source,
        })
}

/// Get the raw JSON value associated with `key` from the client-side state.
fn client_get_raw<'session>(
    state: &'session ClientState,
    flag: &'session InvalidationFlag,
    key: &str,
) -> Option<&'session Value> {
    if flag.is_invalidated() {
        tracing::trace!(
            "Attempted to get a client-side value on a session that's been invalidated."
        );
        return None;
    }
    match state {
        ClientState::Unchanged { state } | ClientState::Updated { state } => state.get(key),
    }
}

fn client_is_empty(state: &ClientState, flag: &InvalidationFlag) -> bool {
    if flag.is_invalidated() {
        return true;
    }
    match state {
        ClientState::Updated { state } | ClientState::Unchanged { state } => state.is_empty(),
    }
}

/// Little helper to create a new `OnceCell` with a value, if provided.
fn new_cell_with<T>(value: Option<T>) -> OnceCell<T> {
    match value {
        Some(t) => OnceCell::from(t),
        None => OnceCell::new(),
    }
}

/// Load the server-side state from the store, then return a mutable reference to it.
async fn force_load_mut<'a>(
    session: &'a mut Session<'_>,
) -> Result<&'a mut ServerState, LoadError> {
    force_load(session).await?;
    let Some(state) = session.server_state.get_mut() else {
        unreachable!("Server-side state should have been loaded by now!")
    };
    Ok(state)
}

/// Load the server-side state from the store, then return an immutable reference to it.
async fn force_load_ref<'a>(session: &'a Session<'_>) -> Result<&'a ServerState, LoadError> {
    force_load(session).await?;
    let Some(state) = session.server_state.get() else {
        unreachable!("Server-side state should have been loaded by now!")
    };
    Ok(state)
}

/// Load the server-side state from the store.
/// This method does nothing if the server-side state has already been loaded.
///
/// After calling this method, the server-side state will be loaded
/// and cached in memory, so that subsequent calls to [`get_raw`](#method.get_raw),
/// [`insert_raw`](#method.insert_raw), and [`remove_raw`](#method.remove_raw)
/// will operate on the in-memory state.
async fn force_load(session: &Session<'_>) -> Result<(), LoadError> {
    // All other cases either imply that we've already loaded the
    // server state or that we don't need to (e.g. delete).
    let Some(session_id) = session.id.old_id() else {
        return Ok(());
    };
    if session.server_state.get().is_some() {
        return Ok(());
    }
    let record = session.store.load(&session_id).await?;
    let mut must_invalidate = false;
    let server_state = match record {
        Some(r) => ServerState::Unchanged {
            state: r.state,
            ttl: r.ttl,
        },
        None => {
            match session.config.state.missing_server_state {
                MissingServerState::Allow => ServerState::DoesNotExist,
                MissingServerState::Reject => {
                    // This can happen in some edge cases—e.g. the state expired between
                    // the time the server received the request and the time it tried to load
                    // the state.
                    must_invalidate = true;
                    ServerState::MarkedForDeletion
                }
            }
        }
    };
    if session.server_state.set(server_state).is_err() {
        tracing::warn!(
            "There were multiple concurrent attempts to load the server-side state for the same session.
            The state loaded by this one will be discarded."
        );
    } else {
        // We invalidate the session here, rather than doing above, because we want to make
        // sure we succeeded in setting the state.
        // If someone else beat us to it, we want to let them make a decision
        // based on the state they loaded.
        // Race conditions all the way down.
        if must_invalidate {
            tracing::warn!(
                "There is no server-side state for the current session, \
                even though one was expected. Invalidating the current session."
            );
            session.invalidated.invalidate();
        }
    }
    Ok(())
}

/// Errors that can occur when interacting with the session state.
pub mod errors {
    use std::borrow::Cow;

    use pavex::{Response, methods};

    use crate::store::errors::{
        ChangeIdError, CreateError, DeleteError, LoadError, UpdateError, UpdateTtlError,
    };

    #[derive(Debug, thiserror::Error)]
    #[non_exhaustive]
    /// The error returned by [`Session::sync`][super::Session::sync].
    pub enum SyncError {
        #[error("Failed to create a new session record")]
        CreateError(#[from] CreateError),
        #[error("Failed to update a session record")]
        UpdateError(#[from] UpdateError),
        #[error("Failed to delete a session record")]
        DeleteError(#[from] DeleteError),
        #[error("Failed to update the TTL for a session record")]
        UpdateTtlError(#[from] UpdateTtlError),
        #[error("Failed to change the session id for a session record")]
        ChangeIdError(#[from] ChangeIdError),
    }

    #[derive(Debug, thiserror::Error)]
    #[non_exhaustive]
    /// The error returned by [`Session::get`][super::Session::get].
    pub enum ServerGetError {
        #[error("Failed to load the session record")]
        LoadError(#[from] LoadError),
        #[error(transparent)]
        DeserializationError(#[from] ValueDeserializationError),
    }

    #[derive(Debug, thiserror::Error)]
    #[non_exhaustive]
    /// The error returned by [`Session::remove`][super::Session::remove].
    pub enum ServerRemoveError {
        #[error("Failed to load the session record")]
        LoadError(#[from] LoadError),
        #[error(transparent)]
        DeserializationError(#[from] ValueDeserializationError),
    }

    #[derive(Debug, thiserror::Error)]
    #[non_exhaustive]
    /// The error returned by [`Session::insert`][super::Session::insert].
    pub enum ServerInsertError {
        #[error("Failed to load the session record")]
        LoadError(#[from] LoadError),
        #[error(transparent)]
        SerializationError(#[from] ValueSerializationError),
    }

    #[derive(Debug, thiserror::Error)]
    #[non_exhaustive]
    #[error(
        "Failed to deserialize the value associated with `{key}` in the {location}-side session state"
    )]
    /// Returned when we fail to deserialize a value stored in either the server or the client
    /// session state.
    pub struct ValueDeserializationError {
        /// The key of the value that we failed to deserialize.
        pub key: Cow<'static, str>,
        pub(crate) location: ValueLocation,
        #[source]
        /// The underlying deserialization error.
        pub(crate) source: serde_json::Error,
    }

    #[derive(Debug, thiserror::Error)]
    #[non_exhaustive]
    #[error(
        "Failed to serialize the value that would have been associated with `{key}` in the {location}-side session state"
    )]
    /// Returned when we fail to serialize a value to be stored in either the server or the client
    /// session state.
    pub struct ValueSerializationError {
        /// The key of the value that we failed to serialize.
        pub key: Cow<'static, str>,
        pub(crate) location: ValueLocation,
        #[source]
        /// The underlying serialization error.
        pub(crate) source: serde_json::Error,
    }

    /// Where the value was stored.
    #[derive(Debug)]
    pub(crate) enum ValueLocation {
        Server,
        Client,
    }

    impl std::fmt::Display for ValueLocation {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let s = match self {
                ValueLocation::Server => "server",
                ValueLocation::Client => "client",
            };
            write!(f, "{s}")
        }
    }

    /// The error returned by [`finalize_session`][crate::finalize_session].
    #[derive(Debug, thiserror::Error)]
    #[non_exhaustive]
    pub enum FinalizeError {
        #[error("Failed to serialize the client-side session state")]
        SerializationError(#[from] serde_json::Error),
        #[error("Failed to sync the server-side session state")]
        SyncErr(#[from] SyncError),
    }

    #[methods]
    impl FinalizeError {
        /// Convert the error into a response.
        #[error_handler]
        pub fn into_response(&self) -> Response {
            Response::internal_server_error()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Session;

    // Check that `Session` is not `Send` nor `Sync`.
    static_assertions::assert_not_impl_any!(Session: Send, Sync);
}
