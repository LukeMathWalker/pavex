use super::state::errors::{ServerGetError, ServerSetError, SyncError, ValueDeserializationError};
use errors::{FinalizeError, ValueSerializationError};
use pavex::cookie::{RemovalCookie, ResponseCookie};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use std::borrow::Cow;
use std::collections::HashMap;

use crate::config::{ServerStateCreation, SessionCookieKind, TtlExtensionTrigger};
use crate::incoming::IncomingSession;
use crate::store::errors::{DeleteError, LoadError};
use crate::store::SessionRecordRef;
use crate::wire::WireClientState;
use crate::SessionConfig;
use crate::SessionId;
use crate::SessionStore;

#[derive(Clone, Debug)]
/// The current HTTP session.
pub struct Session<'store> {
    id: CurrentSessionId,
    server_state: ServerState,
    client_state: ClientState,
    invalidated: bool,
    store: &'store SessionStore,
    config: &'store SessionConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CurrentSessionId {
    Existing(SessionId),
    ToBeRenamed { old: SessionId, new: SessionId },
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
    MarkedForDeletion,
    Unchanged { state: HashMap<String, Value> },
    Updated { state: HashMap<String, Value> },
}

#[derive(Debug, Clone)]
enum ServerState {
    NotLoaded,
    Unchanged {
        state: HashMap<String, Value>,
        ttl: std::time::Duration,
    },
    DoesNotExist,
    MarkedForDeletion,
    Changed {
        state: HashMap<String, Value>,
    },
}

impl<'store> Session<'store> {
    /// Create a new HTTP session.
    ///
    /// It is a continuation of the existing session if there was a valid session cookie
    /// attached to the request.
    /// It is a brand-new session otherwise.
    pub fn new(
        store: &'store SessionStore,
        config: &'store SessionConfig,
        incoming_session: Option<IncomingSession>,
    ) -> Self {
        let (client_state, previous_session_id) = match incoming_session {
            Some(s) => (s.client_state, Some(s.id)),
            None => (Default::default(), None),
        };
        let (id, on_server_sync) = match previous_session_id {
            Some(id) => (CurrentSessionId::Existing(id), ServerState::NotLoaded),
            None => (
                CurrentSessionId::NewlyGenerated(SessionId::random()),
                ServerState::DoesNotExist,
            ),
        };
        Self {
            id,
            server_state: on_server_sync,
            client_state: ClientState::Unchanged {
                state: client_state,
            },
            invalidated: false,
            store,
            config,
        }
    }

    /// Read values from the client-side state attached to this session.
    pub fn client<'session>(&'session self) -> ClientSessionState<'session> {
        ClientSessionState(&self.client_state)
    }

    /// Read or mutate the client-side state attached to this session.
    pub fn client_mut<'session>(&'session mut self) -> ClientSessionStateMut<'session> {
        ClientSessionStateMut(&mut self.client_state)
    }

    /// Read or mutate the server-side state attached to this session.
    pub fn server<'session>(&'session mut self) -> ServerSessionState<'session, 'store> {
        ServerSessionState(self)
    }

    /// Generate a new session identifier and attach it to this session.  
    /// The session state is preserved on both the client-side and the server-side.
    ///
    /// This method is useful for security reasons, as it can help prevent
    /// session fixation attacks.
    pub fn cycle_id(&mut self) {
        let old = match &self.id {
            CurrentSessionId::Existing(id) => Some(*id),
            CurrentSessionId::ToBeRenamed { old, .. } => Some(*old),
            CurrentSessionId::NewlyGenerated(_) => None,
        };
        let new = SessionId::random();

        // Integrity check.
        assert_ne!(
            old,
            Some(new),
            "The newly generated session ID is the same as the old one. This should be impossible."
        );

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
        self.server_state = ServerState::MarkedForDeletion;
        self.client_state = ClientState::MarkedForDeletion;
        self.invalidated = true;
    }

    /// A post-processing middleware to attach a session cookie to the outgoing response, if needed.
    ///
    /// It will also sync the session server-side state with the chosen storage backend.
    pub(crate) async fn finalize(
        &mut self,
    ) -> Result<Option<ResponseCookie<'static>>, FinalizeError> {
        self.server().sync().await?;

        let cookie_config = &self.config.cookie;
        let cookie_name = &cookie_config.name;

        match &self.client_state {
            ClientState::MarkedForDeletion => {
                let mut cookie = RemovalCookie::new(cookie_name.clone());
                if let Some(domain) = cookie_config.domain.as_deref() {
                    cookie = cookie.set_domain(domain.to_owned());
                }
                if let Some(path) = cookie_config.path.as_deref() {
                    cookie = cookie.set_path(path.to_owned());
                }
                Ok(Some(cookie.into()))
            }
            ClientState::Updated {
                state: client_state,
            }
            | ClientState::Unchanged {
                state: client_state,
            } => {
                let server_record_exists = match &self.server_state {
                    ServerState::Unchanged { .. } => Some(true),
                    ServerState::DoesNotExist => Some(false),
                    ServerState::NotLoaded => None,
                    ServerState::MarkedForDeletion | ServerState::Changed { .. } => {
                        unreachable!("The server state has just been synchronized.")
                    }
                };
                // The session is new, we don't have a server-side record, and the client state is empty.
                // We don't need to create a session cookie in this case.
                if self.id.old_id().is_none() && !server_record_exists.unwrap_or(true) {
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
                    let max_age = self
                        .config
                        .state
                        .ttl
                        .try_into()
                        .unwrap_or(time::Duration::MAX);
                    cookie = cookie.set_max_age(max_age);
                }
                Ok(Some(cookie))
            }
        }
    }
}

/// A read-only reference to the client-side state of a session.
pub struct ClientSessionState<'session>(&'session ClientState);

impl<'session> ClientSessionState<'session> {
    /// Get the value associated with `key` from the client-side state.
    ///
    /// If the value is not found, `None` is returned.  
    /// If the value is found, but it cannot be deserialized into the expected type, an error is returned.
    pub fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>, serde_json::Error> {
        self.get_value(key)
            .map(|value| serde_json::from_value(value.clone()))
            .transpose()
    }

    /// Get the raw JSON value associated with `key` from the client-side state.
    pub fn get_value(&self, key: &str) -> Option<&'session Value> {
        match self.0 {
            ClientState::MarkedForDeletion => {
                tracing::trace!(
                    "Attempted to get a client-side value on a session marked for deletion."
                );
                None
            }
            ClientState::Unchanged { state } | ClientState::Updated { state } => state.get(key),
        }
    }
}

/// A mutable reference to the client-side state of a session.
pub struct ClientSessionStateMut<'session>(&'session mut ClientState);

impl<'session> ClientSessionStateMut<'session> {
    /// Set a value in the client-side state for the given key.
    ///
    /// If the key already exists, the value is updated and the old value is returned.
    pub fn set_value(&mut self, key: String, value: Value) -> Option<Value> {
        match &mut self.0 {
            ClientState::MarkedForDeletion => {
                tracing::trace!(
                    "Attempted to set a client-side value on a session marked for deletion."
                );
                None
            }
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
    pub fn remove_value(&mut self, key: &str) -> Option<Value> {
        match &mut self.0 {
            ClientState::MarkedForDeletion => None,
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
        match &mut self.0 {
            ClientState::MarkedForDeletion => {}
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

/// A mutable reference to the server-side state of a session.
pub struct ServerSessionState<'session, 'store>(&'session mut Session<'store>);

impl<'session, 'store> ServerSessionState<'session, 'store> {
    /// Get the value associated with `key` from the server-side state.
    ///
    /// If the value is not found, `None` is returned.  
    /// If the value cannot be deserialized into the expected type, an error is returned.
    pub async fn get<T: DeserializeOwned>(
        &mut self,
        key: &str,
    ) -> Result<Option<T>, ServerGetError> {
        self.get_value(key)
            .await?
            .map(|value| serde_json::from_value(value.clone()))
            .transpose()
            .map_err(|e| {
                ValueDeserializationError {
                    key: key.to_owned(),
                    source: e,
                }
                .into()
            })
    }

    /// Set a value in the server-side state for the given key.
    ///
    /// If the key already exists, the old raw value is returned.  
    /// If the value cannot be serialized, an error is returned.
    pub async fn set<T: Serialize>(
        &mut self,
        key: String,
        value: T,
    ) -> Result<Option<Value>, ServerSetError> {
        let value = serde_json::to_value(value).map_err(|e| ValueSerializationError {
            key: key.clone(),
            source: e,
        })?;
        self.set_value(key, value).await.map_err(Into::into)
    }

    /// Get the value associated with `key` from the server-side state.
    pub async fn get_value(&mut self, key: &str) -> Result<Option<&Value>, LoadError> {
        self.force_load().await?;
        match &self.0.server_state {
            ServerState::MarkedForDeletion => {
                tracing::debug!(session.key = %key, "Tried to access a server-side value on a session marked for deletion.");
                return Ok(None);
            }
            ServerState::NotLoaded => {
                unreachable!("Server state should have been loaded by now.")
            }
            ServerState::Unchanged { state, .. } | ServerState::Changed { state } => {
                Ok(state.get(key))
            }
            ServerState::DoesNotExist => Ok(None),
        }
    }

    /// Set a value in the server-side state for the given key.
    ///
    /// If the key already exists, the old value is returned.
    pub async fn set_value(
        &mut self,
        key: String,
        value: Value,
    ) -> Result<Option<Value>, LoadError> {
        self.force_load().await?;
        let mut existing_state;
        match &mut self.0.server_state {
            ServerState::MarkedForDeletion => {
                tracing::debug!(session.key = %key, "Tried to set a server-side value on a session marked for deletion.");
                return Ok(None);
            }
            ServerState::NotLoaded => {
                unreachable!("Server state should have been loaded by now.")
            }
            ServerState::Unchanged { state, .. } | ServerState::Changed { state } => {
                existing_state = std::mem::take(state);
            }
            ServerState::DoesNotExist => {
                existing_state = HashMap::new();
            }
        };
        let old_value = existing_state.insert(key, value);
        self.0.server_state = ServerState::Changed {
            state: existing_state,
        };
        Ok(old_value)
    }

    /// Remove the value associated with `key` from the server-side state.
    ///
    /// If the key exists, the removed value is returned.
    pub async fn remove_value(&mut self, key: &str) -> Result<Option<Value>, LoadError> {
        self.force_load().await?;
        match &mut self.0.server_state {
            ServerState::MarkedForDeletion => {
                tracing::debug!(session.key = %key, "Tried to delete a server-side value on a session marked for deletion.");
                Ok(None)
            }
            ServerState::DoesNotExist => Ok(None),
            ServerState::NotLoaded => {
                unreachable!("Server state should have been loaded by now.")
            }
            ServerState::Unchanged { state, .. } | ServerState::Changed { state } => {
                Ok(state.remove(key))
            }
        }
    }

    /// Delete the session record from the store.
    ///
    /// This doesn't destroy the whole session—you must invoke [`Session::invalidate`]
    /// if that's your goal.
    pub fn delete(&mut self) {
        self.0.server_state = ServerState::MarkedForDeletion;
    }

    /// Remove all key-value pairs from the server-side state.
    ///
    /// This doesn't delete the session record from the store—you must invoke
    /// [`Session::delete`][Self::delete] if you want to delete the record altogether.
    ///
    /// This doesn't invalidate the session—you must invoke [`Session::invalidate`]
    /// if you want to delete the session altogether.
    pub async fn clear(&mut self) -> Result<(), LoadError> {
        self.force_load().await?;
        match &mut self.0.server_state {
            ServerState::MarkedForDeletion | ServerState::DoesNotExist => {}
            ServerState::NotLoaded => {
                unreachable!("Server state should have been loaded by now.")
            }
            ServerState::Unchanged { state, .. } => {
                if !state.is_empty() {
                    self.0.server_state = ServerState::Changed {
                        state: HashMap::new(),
                    };
                }
            }
            ServerState::Changed { state } => {
                state.clear();
            }
        }
        Ok(())
    }

    /// Sync the in-memory representation of the server-side state
    /// with the store.
    ///
    /// In most cases, you don't need to invoke this method manually: it is
    /// done for you by [`finalize_session`][`super::finalize_session`],
    /// the post-processing middleware that attaches the session cookie to
    /// the response returned to the client.
    pub async fn sync(&mut self) -> Result<(), SyncError> {
        let state_config = &self.0.config.state;
        let fresh_ttl = state_config.ttl;
        let create_if_empty = {
            let has_client_side = self.0.id.old_id().is_some()
                || matches!(self.0.client_state, ClientState::Updated { .. });
            has_client_side && state_config.server_state_creation == ServerStateCreation::NeverSkip
        };
        match &self.0.server_state {
            ServerState::DoesNotExist => match self.0.id {
                CurrentSessionId::NewlyGenerated(id) | CurrentSessionId::Existing(id) => {
                    if create_if_empty {
                        self.0
                            .store
                            .create(&id, SessionRecordRef::empty(fresh_ttl))
                            .await?;
                    }
                }
                CurrentSessionId::ToBeRenamed { .. } => {
                    // Nothing to do.
                }
            },
            ServerState::NotLoaded => {
                match self.0.id {
                    CurrentSessionId::Existing(_) => {
                        // Nothing to do.
                    }
                    CurrentSessionId::ToBeRenamed { old, new } => {
                        if old != new {
                            self.0.store.change_id(&old, &new).await?;
                        }
                    }
                    CurrentSessionId::NewlyGenerated(..) => {
                        unreachable!("A newly generated session cannot have a 'NotLoaded' server state. It must be set to 'DoesNotExist'.")
                    }
                };
            }
            ServerState::Unchanged {
                state,
                ttl: remaining_ttl,
            } => {
                match self.0.id {
                    CurrentSessionId::Existing(old) => {
                        if state_config.extend_ttl == TtlExtensionTrigger::OnStateLoadsAndChanges {
                            let extend = state_config
                                .ttl_extension_threshold
                                .map(|ratio| *remaining_ttl < fresh_ttl.mul_f32(ratio.inner()))
                                .unwrap_or(true);
                            if extend {
                                self.0.store.update_ttl(&old, fresh_ttl).await?;
                            }
                        }
                    }
                    CurrentSessionId::ToBeRenamed { old, new } => {
                        if old != new {
                            // TODO: introduce a faster rename operation
                            self.0.store.delete(&old).await?;
                            let record = SessionRecordRef {
                                state: Cow::Borrowed(state),
                                ttl: fresh_ttl,
                            };
                            self.0.store.create(&new, record).await?;
                        }
                    }
                    CurrentSessionId::NewlyGenerated(new) => {
                        if create_if_empty {
                            self.0
                                .store
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
            ServerState::MarkedForDeletion => match self.0.id.old_id() {
                Some(id) => {
                    if let Err(e) = self.0.store.delete(&id).await {
                        match e {
                            // We're good as long as we made sure that no server-side
                            // state is stored against this id, we're good.
                            DeleteError::UnknownId(_) => {}
                            _ => return Err(e.into()),
                        }
                    }
                }
                None => {
                    tracing::trace!("The server session state was marked for deletion, but there was no session to delete. This is a no-op.")
                }
            },
            ServerState::Changed { state } => {
                let record = SessionRecordRef {
                    state: Cow::Borrowed(state),
                    ttl: fresh_ttl,
                };
                match self.0.id {
                    CurrentSessionId::Existing(id) => {
                        self.0.store.update(&id, record).await?;
                    }
                    CurrentSessionId::ToBeRenamed { old, new } => {
                        if old != new {
                            if let Err(e) = self.0.store.delete(&old).await {
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
                            self.0.store.create(&new, record).await?;
                        } else {
                            self.0.store.update(&old, record).await?;
                        }
                    }
                    CurrentSessionId::NewlyGenerated(id) => {
                        self.0.store.create(&id, record).await?;
                    }
                }
            }
        };

        let new_self = Session {
            id: self.0.id.clone(),
            server_state: {
                // The value we use here as replacement doesn't matter, because we're going to throw away
                // the old `self` anyway. We use `MarkedForDeletion` because it's free to create.
                let old_state =
                    std::mem::replace(&mut self.0.server_state, ServerState::MarkedForDeletion);
                match old_state {
                    ServerState::Changed { state } => ServerState::Unchanged {
                        state,
                        ttl: fresh_ttl,
                    },
                    ServerState::Unchanged { state, ttl } => ServerState::Unchanged { state, ttl },
                    ServerState::MarkedForDeletion => ServerState::DoesNotExist,
                    ServerState::NotLoaded => ServerState::NotLoaded,
                    ServerState::DoesNotExist => {
                        if create_if_empty {
                            ServerState::Unchanged {
                                state: HashMap::new(),
                                ttl: fresh_ttl,
                            }
                        } else {
                            ServerState::DoesNotExist
                        }
                    }
                }
            },
            client_state: {
                // The value we use here as replacement doesn't matter, because we're going to throw away
                // the old `self` anyway. We use `MarkedForDeletion` because it's free to create.
                std::mem::replace(&mut self.0.client_state, ClientState::MarkedForDeletion)
            },
            store: self.0.store,
            invalidated: self.0.invalidated,
            config: self.0.config,
        };
        *self.0 = new_self;
        Ok(())
    }

    /// Load the server-side state from the store.  
    /// This method does nothing if the server-side state has already been loaded.
    ///
    /// After calling this method, the server-side state will be loaded
    /// and cached in memory, so that subsequent calls to [`get_value`](#method.get_value),
    /// [`set_value`](#method.set_value), and [`remove_value`](#method.remove_value)
    /// will operate on the in-memory state.
    pub async fn force_load(&mut self) -> Result<(), LoadError> {
        // All other cases either imply that we've already loaded the
        // server state or that we don't need to (e.g. delete).
        if !matches!(self.0.server_state, ServerState::NotLoaded) {
            return Ok(());
        }
        let Some(session_id) = self.0.id.old_id() else {
            return Ok(());
        };
        let record = self.0.store.load(&session_id).await?;
        let on_server_sync = match record {
            Some(r) => ServerState::Unchanged {
                state: r.state,
                ttl: r.ttl,
            },
            None => {
                if self.0.config.state.server_state_creation == ServerStateCreation::SkipIfEmpty {
                    ServerState::DoesNotExist
                } else {
                    // This should never happen, as we should have created the server state
                    // when the session was created, even if it was empty.
                    // It can happen in some edge cases, like if the state expired between
                    // the time the server received the request and the time it tried to load
                    // the state, but it's still a condition that should be raised to the
                    // developer.
                    tracing::warn!(
                        "There is no server-side state for the current session, \
                        even though one was expected. Invalidating the session."
                    );
                    self.0.invalidate();
                    return Ok(());
                }
            }
        };
        self.0.server_state = on_server_sync;
        Ok(())
    }
}

/// Errors that can occur when interacting with the session state.
pub mod errors {
    use pavex::response::Response;

    use crate::store::errors::{
        ChangeIdError, CreateError, DeleteError, LoadError, UpdateError, UpdateTtlError,
    };

    #[derive(Debug, thiserror::Error)]
    #[non_exhaustive]
    /// The error returned by [`ServerSessionState::sync`][super::ServerSessionState::sync].
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
    /// The error returned by [`ServerSessionState::get`][super::ServerSessionState::get].
    pub enum ServerGetError {
        #[error("Failed to load the session record")]
        LoadError(#[from] LoadError),
        #[error(transparent)]
        DeserializationError(#[from] ValueDeserializationError),
    }

    #[derive(Debug, thiserror::Error)]
    #[non_exhaustive]
    /// The error returned by [`ServerSessionState::set`][super::ServerSessionState::set].
    pub enum ServerSetError {
        #[error("Failed to load the session record")]
        LoadError(#[from] LoadError),
        #[error(transparent)]
        SerializationError(#[from] ValueSerializationError),
    }

    #[derive(Debug, thiserror::Error)]
    #[non_exhaustive]
    #[error("Failed to deserialize the value associated with `{key}`")]
    /// One of the errors returned by [`ServerSessionState::get`][super::ServerSessionState::get].
    pub struct ValueDeserializationError {
        /// The key of the value that we failed to deserialize.
        pub key: String,
        #[source]
        /// The underlying deserialization error.
        pub source: serde_json::Error,
    }

    #[derive(Debug, thiserror::Error)]
    #[non_exhaustive]
    #[error("Failed to serialize the value associated with `{key}`")]
    /// One of the errors returned by [`ServerSessionState::set`][super::ServerSessionState::set].
    pub struct ValueSerializationError {
        /// The key of the value that we failed to serialize.
        pub key: String,
        #[source]
        /// The underlying serialization error.
        pub source: serde_json::Error,
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

    impl FinalizeError {
        /// Convert the error into a response.
        pub fn into_response(&self) -> Response {
            Response::internal_server_error()
        }
    }
}
