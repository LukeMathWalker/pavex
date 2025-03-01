use std::{collections::HashMap, num::NonZeroUsize};

use assertions::is_removal_cookie;
use fixtures::{SessionFixture, spy_store, store};
use googletest::{
    assert_that,
    prelude::{empty, eq, len, none, not},
};
use helpers::SetCookie;
use itertools::Itertools;
use pavex_session::{
    IncomingSession, Session, SessionConfig, SessionId,
    config::{MissingServerState, ServerStateCreation, TtlExtensionTrigger},
};

mod assertions;
mod config;
mod fixtures;
mod helpers;
mod operations;

#[tokio::test]
async fn id_can_be_cycled_for_a_fresh_session() {
    let ((store, call_tracker), config) = (spy_store(), SessionConfig::default());

    let mut session = Session::new(&store, &config, None);
    // Nothing really happens, but it doesn't error out.
    session.cycle_id();

    let cookie = session.finalize().await.unwrap();
    assert_that!(cookie, none());

    call_tracker.assert_store_was_untouched().await;
}

#[tokio::test]
async fn untouched_session_is_not_sent_to_the_client() {
    let ((store, call_tracker), config) = (spy_store(), SessionConfig::default());

    let mut session = Session::new(&store, &config, None);
    let cookie = session.finalize().await.unwrap();
    assert_that!(cookie, none());

    call_tracker.assert_store_was_untouched().await;
}

#[tokio::test]
async fn no_removal_cookie_is_sent_for_a_fresh_but_invalidated_session() {
    let ((store, call_tracker), config) = (spy_store(), SessionConfig::default());
    let mut session = Session::new(&store, &config, None);

    let key = "my_key";
    session.client_mut().insert(key.to_owned(), "hey").unwrap();
    session.insert(key.to_owned(), "yo").await.unwrap();

    session.invalidate();

    assert!(session.is_invalidated());

    let cookie = session.finalize().await.unwrap();
    assert_that!(cookie, none());

    // Since the session was invalidated, and there was no prior state,
    // we didn't perform any calls to the server store.
    call_tracker.assert_store_was_untouched().await;

    // Session is still treated as invalidated after syncing.
    assert!(session.is_invalidated());
}

#[tokio::test]
async fn removal_cookie_is_sent_if_existing_session_is_invalidated() {
    let (store, config) = (store(), SessionConfig::default());

    let incoming = IncomingSession::from_parts(SessionId::random(), HashMap::new());
    let mut session = Session::new(&store, &config, Some(incoming));

    session.invalidate();

    assert!(session.is_invalidated());

    let cookie = session.finalize().await.unwrap().unwrap();
    assert_that!(cookie, is_removal_cookie());

    // Session is still treated as invalidated after syncing.
    assert!(session.is_invalidated());
}

#[tokio::test]
async fn pre_existing_client_state_can_be_accessed() {
    let (store, config) = (store(), SessionConfig::default());

    let mut fixture = SessionFixture::default();
    let key = "key";
    let value = serde_json::Value::String("Value".to_owned());
    fixture.client_state = {
        let mut c = HashMap::new();
        c.insert(key.into(), value.clone());
        c
    };

    let incoming = fixture.setup(&store).await;
    let mut session = Session::new(&store, &config, Some(incoming));

    assert_eq!(session.client().get_raw(key).unwrap(), &value);
    // No source confusion!
    assert!(session.is_empty().await.unwrap());

    let cookie = session.finalize().await.unwrap().unwrap();
    let cookie = SetCookie::parse(cookie);
    assert_eq!(cookie.id(), fixture.id());
    // The original value is still there.
    assert_eq!(cookie.client_state, fixture.client_state);
}

#[tokio::test]
async fn pre_existing_client_state_can_be_modified() {
    let (store, config) = (store(), SessionConfig::default());

    let mut fixture = SessionFixture::default();
    let key = "key";
    let value = serde_json::Value::String("Value".to_owned());
    fixture.client_state = {
        let mut c = HashMap::new();
        c.insert(key.into(), value.clone());
        c
    };

    let incoming = fixture.setup(&store).await;
    let mut session = Session::new(&store, &config, Some(incoming));

    assert_eq!(session.client().get_raw(key).unwrap(), &value);
    session.client_mut().remove_raw(key);

    let cookie = session.finalize().await.unwrap().unwrap();
    let cookie = SetCookie::parse(cookie);
    assert_eq!(cookie.id(), fixture.id());
    // The original value is no longer there.
    assert!(cookie.client_state.is_empty());
}

#[tokio::test]
async fn pre_existing_client_state_can_be_cleared_without_invalidating_the_session() {
    let (store, config) = (store(), SessionConfig::default());
    let mut fixture = SessionFixture::default();
    fixture.client_state = {
        // TODO: generate random client state.
        let mut c = HashMap::new();
        c.insert(
            "a key".into(),
            serde_json::Value::String("Value".to_owned()),
        );
        c
    };

    let incoming = fixture.setup(&store).await;
    let mut session = Session::new(&store, &config, Some(incoming));

    assert!(!session.client().is_empty());
    session.client_mut().clear();

    let cookie = session.finalize().await.unwrap().unwrap();
    let cookie = SetCookie::parse(cookie);
    assert_eq!(cookie.id(), fixture.id());
    // The original values are no longer there.
    assert!(cookie.client_state.is_empty());
}

#[tokio::test]
async fn server_state_can_be_cleared_without_invalidating_the_session() {
    let (store, config) = (store(), SessionConfig::default());

    let mut fixture = SessionFixture::default();
    fixture.server_state = Some({
        // TODO: generate random server state.
        let mut c = HashMap::new();
        c.insert(
            "a key".into(),
            serde_json::Value::String("Value".to_owned()),
        );
        c
    });

    let incoming = fixture.setup(&store).await;
    let mut session = Session::new(&store, &config, Some(incoming));

    assert!(session.client().is_empty());
    assert!(!session.is_empty().await.unwrap());

    session.clear().await.unwrap();

    let cookie = session.finalize().await.unwrap().unwrap();

    // It's not a removal cookie!
    let cookie = SetCookie::parse(cookie);
    assert_eq!(cookie.id(), fixture.id());

    // The server state is present, but empty.
    let server_state = store.load(&fixture.id).await.unwrap().unwrap();
    assert_that!(server_state.state, empty());
}

#[tokio::test]
async fn store_is_not_touched_if_you_clear_an_empty_server_state_and_ttl_is_configured_to_update_on_changes()
 {
    let ((store, call_tracker), mut config) = (spy_store(), SessionConfig::default());
    config.state.extend_ttl = TtlExtensionTrigger::OnStateChanges;

    let fixture = SessionFixture::default();
    let incoming = fixture.setup(&store).await;
    let mut session = Session::new(&store, &config, Some(incoming));
    assert!(session.is_empty().await.unwrap());
    // Otherwise `create` and `load` will show up in the operation log.
    call_tracker.reset_operation_log().await;

    session.clear().await.unwrap();

    let cookie = session.finalize().await.unwrap().unwrap();

    // It's not a removal cookie!
    let cookie = SetCookie::parse(cookie);
    assert_eq!(cookie.id(), fixture.id());

    call_tracker.assert_store_was_untouched().await;
}

#[tokio::test]
async fn ttl_is_updated_if_server_state_is_loaded_but_unchanged() {
    let ((store, call_tracker), mut config) = (spy_store(), SessionConfig::default());
    // Always extend TTL
    config.state.ttl_extension_threshold = None;

    let mut fixture = SessionFixture::default();
    fixture.server_ttl = Some(config.state.ttl);
    let incoming = fixture.setup(&store).await;
    let mut session = Session::new(&store, &config, Some(incoming));
    assert!(session.is_empty().await.unwrap());
    // Otherwise `create` and `load` will show up in the operation log.
    call_tracker.reset_operation_log().await;

    let cookie = session.finalize().await.unwrap().unwrap();

    // It's not a removal cookie!
    let cookie = SetCookie::parse(cookie);
    assert_eq!(cookie.id(), fixture.id());

    let oplog = call_tracker.operation_log().await;
    assert_that!(oplog, len(eq(1)));
    assert!(oplog[0].starts_with("update-ttl"));
}

#[tokio::test]
async fn ttl_is_not_updated_if_server_state_is_unchanged_but_ttl_threshold_is_not_met() {
    let ((store, call_tracker), config) = (spy_store(), SessionConfig::default());
    let ttl_extension_threshold = config.state.ttl_extension_threshold.unwrap();
    assert!(ttl_extension_threshold.inner() < 0.9);

    let mut fixture = SessionFixture::default();
    // We start at full TTL
    fixture.server_ttl = Some(config.state.ttl);
    let incoming = fixture.setup(&store).await;
    let mut session = Session::new(&store, &config, Some(incoming));
    assert!(session.is_empty().await.unwrap());
    // Otherwise `create` and `load` will show up in the operation log.
    call_tracker.reset_operation_log().await;

    let cookie = session.finalize().await.unwrap().unwrap();

    // It's not a removal cookie!
    let cookie = SetCookie::parse(cookie);
    assert_eq!(cookie.id(), fixture.id());

    let oplog = call_tracker.operation_log().await;
    assert_that!(oplog, empty());
}

#[tokio::test]
async fn ttl_is_updated_if_server_state_is_unchanged_and_ttl_threshold_is_met() {
    let ((store, call_tracker), config) = (spy_store(), SessionConfig::default());
    let ttl_extension_threshold = config.state.ttl_extension_threshold.unwrap();

    let mut fixture = SessionFixture::default();
    // We start below the threshold
    let ttl = config.state.ttl.as_secs_f32() * (ttl_extension_threshold.inner() - 0.1);
    assert!(ttl > 0.);
    fixture.server_ttl = Some(std::time::Duration::from_secs_f32(ttl));
    let incoming = fixture.setup(&store).await;
    let mut session = Session::new(&store, &config, Some(incoming));
    assert!(session.is_empty().await.unwrap());
    // Otherwise `create` and `load` will show up in the operation log.
    call_tracker.reset_operation_log().await;

    let cookie = session.finalize().await.unwrap().unwrap();

    // It's not a removal cookie!
    let cookie = SetCookie::parse(cookie);
    assert_eq!(cookie.id(), fixture.id());

    let oplog = call_tracker.operation_log().await;
    assert_that!(oplog, len(eq(1)));
    assert!(oplog[0].starts_with("update-ttl"));
}

#[tokio::test]
async fn server_state_can_be_deleted_without_invalidating_the_session() {
    let ((store, call_tracker), config) = (spy_store(), SessionConfig::default());

    let mut fixture = SessionFixture::default();
    fixture.server_state = Some({
        // TODO: generate random server state.
        let mut c = HashMap::new();
        c.insert(
            "a key".into(),
            serde_json::Value::String("Value".to_owned()),
        );
        c
    });

    let incoming = fixture.setup(&store).await;
    let mut session = Session::new(&store, &config, Some(incoming));

    session.delete();

    let cookie = session.finalize().await.unwrap().unwrap();

    // It's not a removal cookie!
    let cookie = SetCookie::parse(cookie);
    assert_eq!(cookie.id(), fixture.id());

    // If we go straight for a deletion, the server is never loaded in the first place.
    call_tracker.assert_never_loaded().await;

    // The server state is not there anymore.
    let server_state = store.load(&fixture.id).await.unwrap();
    assert!(server_state.is_none());
}

#[tokio::test]
async fn server_state_is_deleted_if_the_session_is_invalidated() {
    let (store, config) = (store(), SessionConfig::default());

    let mut fixture = SessionFixture::default();
    fixture.server_state = Some({
        // TODO: generate random server state.
        let mut c = HashMap::new();
        c.insert(
            "a key".into(),
            serde_json::Value::String("Value".to_owned()),
        );
        c
    });

    let incoming = fixture.setup(&store).await;
    let mut session = Session::new(&store, &config, Some(incoming));

    assert!(session.client().is_empty());
    assert!(!session.is_empty().await.unwrap());

    session.invalidate();

    let cookie = session.finalize().await.unwrap().unwrap();
    assert_that!(cookie, is_removal_cookie());

    // The server state is not there anymore.
    let server_state = store.load(&fixture.id).await.unwrap();
    assert!(server_state.is_none());
}

#[tokio::test]
async fn server_state_is_persisted_for_a_fresh_session() {
    let (store, config) = (store(), SessionConfig::default());
    let mut session = Session::new(&store, &config, None);

    session.insert("key", "value").await.unwrap();

    let cookie = session.finalize().await.unwrap().unwrap();
    let cookie = SetCookie::parse(cookie);

    // The server state is not there anymore.
    let record = store.load(&cookie.id).await.unwrap().unwrap();
    assert_eq!(
        record
            .state
            .into_iter()
            .map(|(k, v)| format!("{k}: {v}"))
            .join("\n"),
        r#"key: "value""#
    );
}

#[tokio::test]
async fn server_state_is_not_created_if_empty_when_config_demands_it() {
    let mut config = SessionConfig::default();
    config.state.server_state_creation = ServerStateCreation::SkipIfEmpty;
    let (store, tracker) = spy_store();
    let mut session = Session::new(&store, &config, None);

    // Set a client-side value to force session creation.
    session.client_mut().insert_raw("key", "value".into());

    let cookie = session.finalize().await.unwrap().unwrap();
    let _ = SetCookie::parse(cookie);

    // The server state is not created nor loaded at any point.
    tracker.assert_store_was_untouched().await;
}

#[tokio::test]
async fn server_state_is_created_if_empty_when_config_demands_it() {
    let mut config = SessionConfig::default();
    config.state.server_state_creation = ServerStateCreation::NeverSkip;
    let store = store();
    let mut session = Session::new(&store, &config, None);

    // Set a client-side value to force session creation.
    session.client_mut().insert_raw("key", "value".into());

    let cookie = session.finalize().await.unwrap().unwrap();
    // Not a removal cookie.
    let cookie = SetCookie::parse(cookie);

    // The server state is created.
    let server_state = store.load(&cookie.id).await.unwrap();
    assert!(server_state.is_some());
}

#[tokio::test]
async fn server_state_for_existing_session_can_be_missing_if_config_allows_it() {
    let mut config = SessionConfig::default();
    config.state.missing_server_state = MissingServerState::Allow;

    let store = store();
    let incoming = SessionFixture {
        server_state: None,
        ..Default::default()
    }
    .setup(&store)
    .await;
    let mut session = Session::new(&store, &config, Some(incoming));

    session.force_load().await.unwrap();

    let cookie = session.finalize().await.unwrap().unwrap();
    // Not a removal cookie, the session wasn't considered invalid.
    assert_that!(cookie, not(is_removal_cookie()));
}

#[tokio::test]
async fn server_state_for_existing_session_cannot_be_missing_if_config_forbids_it() {
    let mut config = SessionConfig::default();
    config.state.missing_server_state = MissingServerState::Reject;

    let store = store();
    let incoming = SessionFixture {
        server_state: None,
        ..Default::default()
    }
    .setup(&store)
    .await;
    let mut session = Session::new(&store, &config, Some(incoming));

    // Force loading the server state, thus allowing us to realize it's not there.
    session.force_load().await.unwrap();

    // The session is invalidated.
    let cookie = session.finalize().await.unwrap().unwrap();
    assert_that!(cookie, is_removal_cookie());
}

#[tokio::test]
async fn server_state_is_created_even_if_empty_with_default_config() {
    let config = SessionConfig::default();

    let store = store();
    let mut session = Session::new(&store, &config, None);

    // Add client-side state to force session creation.
    session.client_mut().insert("key", "value").unwrap();

    let cookie = session.finalize().await.unwrap().unwrap();
    let cookie = SetCookie::parse(cookie);

    let server_state = store.load(&cookie.id).await.unwrap();
    assert!(server_state.is_some());
}

#[tokio::test]
async fn server_state_is_not_created_when_empty_if_config_allows_it() {
    let mut config = SessionConfig::default();
    config.state.server_state_creation = ServerStateCreation::SkipIfEmpty;

    let store = store();
    let mut session = Session::new(&store, &config, None);

    // Add client-side state to force session creation.
    session.client_mut().insert("key", "value").unwrap();

    let cookie = session.finalize().await.unwrap().unwrap();
    let cookie = SetCookie::parse(cookie);

    let server_state = store.load(&cookie.id).await.unwrap();
    assert!(server_state.is_none());
}

#[tokio::test]
async fn session_id_of_an_existing_session_changes_if_cycled() {
    let (store, config) = (store(), SessionConfig::default());

    let fixture = SessionFixture::default();
    let original_session_id = fixture.id();
    let incoming = fixture.setup(&store).await;
    let mut session = Session::new(&store, &config, Some(incoming));

    session.cycle_id();

    let cookie = session.finalize().await.unwrap().unwrap();
    let cookie = SetCookie::parse(cookie);
    assert_ne!(cookie.id(), original_session_id);
}

#[tokio::test]
async fn id_cycling_fails_if_the_old_state_record_is_gone_and_it_had_not_been_loaded_previously() {
    let (store, config) = (store(), SessionConfig::default());

    let fixture = SessionFixture::default();
    let original_session_id = fixture.id;
    let incoming = fixture.setup(&store).await;
    let mut session = Session::new(&store, &config, Some(incoming));

    session.cycle_id();

    // Remove the old state.
    store.delete(&original_session_id).await.unwrap();

    let error = session.finalize().await.unwrap_err();
    assert_that!(
        error.to_string(),
        eq("Failed to sync the server-side session state")
    );
    assert!(error.into_response().status().is_server_error());
}

#[tokio::test]
async fn id_cycling_succeeds_if_the_old_state_record_is_gone_and_but_the_state_had_been_loaded_previously()
 {
    let (store, config) = (store(), SessionConfig::default());

    let fixture = SessionFixture::default();
    let original_session_id = fixture.id;
    let incoming = fixture.setup(&store).await;
    let mut session = Session::new(&store, &config, Some(incoming));

    session.force_load().await.unwrap();
    session.cycle_id();

    // Remove the old state.
    store.delete(&original_session_id).await.unwrap();

    let cookie = session.finalize().await.unwrap().unwrap();
    let cookie = SetCookie::parse(cookie);
    assert_ne!(cookie.id(), original_session_id.inner());
}

#[tokio::test]
async fn id_cycling_succeeds_if_the_old_state_record_is_gone_and_but_the_state_had_been_changed_previously()
 {
    let (store, config) = (store(), SessionConfig::default());

    let fixture = SessionFixture::default();
    let original_session_id = fixture.id;
    let incoming = fixture.setup(&store).await;
    let mut session = Session::new(&store, &config, Some(incoming));

    session.insert("yo", "yo").await.unwrap();
    session.cycle_id();

    // Remove the old state.
    store.delete(&original_session_id).await.unwrap();

    let cookie = session.finalize().await.unwrap().unwrap();
    let cookie = SetCookie::parse(cookie);
    assert_ne!(cookie.id(), original_session_id.inner());
}

#[tokio::test]
async fn server_state_is_reassigned_when_session_id_changes() {
    let (store, config) = (store(), SessionConfig::default());

    let fixture = SessionFixture::default();
    let original_session_id = fixture.id;
    let incoming = fixture.setup(&store).await;
    let mut session = Session::new(&store, &config, Some(incoming));

    session.force_load().await.unwrap();
    session.cycle_id();

    let cookie = session.finalize().await.unwrap().unwrap();

    let cookie = SetCookie::parse(cookie);
    assert_ne!(cookie.id(), original_session_id.inner());

    assert!(store.load(&cookie.id).await.unwrap().is_some());
    assert!(store.load(&original_session_id).await.unwrap().is_none());
}

#[tokio::test]
async fn store_is_not_hit_if_you_try_to_load_the_server_state_for_a_fresh_session() {
    let ((store, call_tracker), config) = (spy_store(), SessionConfig::default());
    let session = Session::new(&store, &config, None);

    session.force_load().await.unwrap();
    call_tracker.assert_never_loaded().await;
}

#[tokio::test]
async fn new_server_state_is_stored_against_the_new_id_when_cycled() {
    let (store, config) = (store(), SessionConfig::default());

    let fixture = SessionFixture::default();
    let original_session_id = fixture.id;
    let incoming = fixture.setup(&store).await;
    let mut session = Session::new(&store, &config, Some(incoming));

    session.insert_raw("key", "value".into()).await.unwrap();
    session.cycle_id();

    let cookie = session.finalize().await.unwrap().unwrap();

    let cookie = SetCookie::parse(cookie);
    assert_ne!(cookie.id(), original_session_id.inner());

    assert!(store.load(&cookie.id).await.unwrap().is_some());
    assert_that!(store.load(&original_session_id).await.unwrap(), none());
}

#[tokio::test]
async fn session_id_can_be_cycled_multiple_times() {
    let (store, config) = (store(), SessionConfig::default());

    let fixture = SessionFixture::default();
    let original_session_id = fixture.id;
    let incoming = fixture.setup(&store).await;
    let mut session = Session::new(&store, &config, Some(incoming));

    session.cycle_id();
    // Nothing happens now, until the session gets finalized.
    session.cycle_id();
    session.cycle_id();

    let cookie = session.finalize().await.unwrap().unwrap();
    let cookie = SetCookie::parse(cookie);
    assert_that!(cookie.id(), not(eq(original_session_id.inner())));
}

#[tokio::test]
async fn session_debug_representation_does_not_leak_session_id() {
    let (store, config) = (store(), SessionConfig::default());
    let fixture = SessionFixture::default();
    let incoming = fixture.setup(&store).await;
    let session = Session::new(&store, &config, Some(incoming));

    let debug = format!("{session:?}");
    assert!(!debug.contains(&fixture.id.inner().to_string()));
}

#[tokio::test]
async fn delete_expired_delegates_to_the_store_as_expected() {
    let (store, call_tracker) = spy_store();

    let batch_size = NonZeroUsize::try_from(2).unwrap();
    store.delete_expired(Some(batch_size)).await.unwrap();

    store.delete_expired(None).await.unwrap();

    assert_eq!(
        call_tracker.operation_log().await,
        vec!["delete-expired 2", "delete-expired"]
    );
}
