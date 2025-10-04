use pavex_session::SessionId;
use pavex_session::store::{SessionRecordRef, SessionStorageBackend};
use pavex_session_sqlx::MySqlSessionStore;
use serde_json;
use sqlx::mysql::MySqlPoolOptions;
use sqlx::{Connection, MySqlConnection};
use std::borrow::Cow;
use std::collections::HashMap;

use std::time::Duration;

async fn create_test_store() -> MySqlSessionStore {
    let database_url = std::env::var("TEST_MYSQL_URL")
        .unwrap_or_else(|_| "mysql://test:test@localhost:53306/session_test".to_string());

    let pool_options = MySqlPoolOptions::new().acquire_timeout(Duration::from_secs(3));
    let pool = pool_options
        .connect_lazy(&database_url)
        .expect("MySQL test database not available. Set TEST_MYSQL_URL environment variable.");

    let store = MySqlSessionStore::new(pool);
    store.migrate().await.unwrap();

    store
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

    // Verify all data is preserved correctly by comparing with original
    for (key, expected_value) in &state {
        assert_eq!(
            loaded_record.state.get(key).unwrap(),
            expected_value,
            "Mismatch for key: {}",
            key
        );
    }

    // Verify we have the same number of keys
    assert_eq!(loaded_record.state.len(), state.len());
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

    // Load and verify updates
    let loaded_record = store.load(&session_id).await.unwrap().unwrap();

    // Verify all updated data is preserved correctly by comparing with updated state
    for (key, expected_value) in &updated_state {
        assert_eq!(
            loaded_record.state.get(key).unwrap(),
            expected_value,
            "Mismatch for updated key: {}",
            key
        );
    }

    // Verify we have the same number of keys
    assert_eq!(loaded_record.state.len(), updated_state.len());
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

    // Verify TTL was updated but data preserved
    let loaded_record = store.load(&session_id).await.unwrap().unwrap();

    // Verify original data is preserved by comparing with original state
    for (key, expected_value) in &state {
        assert_eq!(
            loaded_record.state.get(key).unwrap(),
            expected_value,
            "Mismatch for key after TTL update: {}",
            key
        );
    }

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

    // Verify all data was transferred to new session ID
    for (key, expected_value) in &state {
        assert_eq!(
            loaded_record.state.get(key).unwrap(),
            expected_value,
            "Mismatch for key after ID change: {}",
            key
        );
    }
}

#[tokio::test]
async fn test_delete_expired() {
    let store = create_test_store().await;

    // First clean up any existing expired sessions
    store.delete_expired(None).await.unwrap();

    // Create a session that expires quickly
    let (expired_session_id, expired_state) = create_test_record(1);
    let expired_record = SessionRecordRef {
        state: Cow::Borrowed(&expired_state),
        ttl: Duration::from_secs(1),
    };
    store
        .create(&expired_session_id, expired_record)
        .await
        .unwrap();

    // Create a session that doesn't expire
    let (valid_session_id, valid_state) = create_test_record(3600);
    let valid_record = SessionRecordRef {
        state: Cow::Borrowed(&valid_state),
        ttl: Duration::from_secs(3600),
    };
    store.create(&valid_session_id, valid_record).await.unwrap();

    // Wait for the first to expire
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Verify expired session can't be loaded
    assert!(store.load(&expired_session_id).await.unwrap().is_none());
    // Verify valid session can still be loaded
    assert!(store.load(&valid_session_id).await.unwrap().is_some());

    // Delete expired sessions - should delete some (at least our expired one)
    let deleted_count = store.delete_expired(None).await.unwrap();
    assert!(
        deleted_count > 0,
        "Should have deleted at least one session"
    );

    // Run again - should delete 0 (all expired sessions already deleted)
    let deleted_count_2 = store.delete_expired(None).await.unwrap();
    assert_eq!(deleted_count_2, 0);

    // Valid session should still exist
    assert!(store.load(&valid_session_id).await.unwrap().is_some());
}

#[tokio::test]
async fn test_delete_expired_with_batch_size() {
    let store = create_test_store().await;

    // First clean up any existing expired sessions
    store.delete_expired(None).await.unwrap();

    // Create 3 sessions that will expire quickly
    let mut expired_session_ids = Vec::new();
    for _ in 0..3 {
        let (session_id, state) = create_test_record(1);
        expired_session_ids.push(session_id.clone());
        let record = SessionRecordRef {
            state: Cow::Borrowed(&state),
            ttl: Duration::from_secs(1),
        };
        store.create(&session_id, record).await.unwrap();
    }

    // Wait for expiration
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Verify sessions are expired before testing batch deletion
    for session_id in &expired_session_ids {
        assert!(store.load(session_id).await.unwrap().is_none());
    }

    // Test batch deletion with batch size of 2
    let batch_size = std::num::NonZeroUsize::new(2).unwrap();

    // First batch - should delete up to 2 sessions
    let deleted_1 = store.delete_expired(Some(batch_size)).await.unwrap();
    assert!(
        deleted_1 <= 2,
        "Batch size not respected: deleted {} but limit was 2",
        deleted_1
    );

    // If there were no expired sessions to delete, create one more test
    if deleted_1 == 0 {
        // Create and immediately expire a session to test the mechanism
        let (test_session_id, test_state) = create_test_record(1);
        let test_record = SessionRecordRef {
            state: Cow::Borrowed(&test_state),
            ttl: Duration::from_secs(1),
        };
        store.create(&test_session_id, test_record).await.unwrap();
        tokio::time::sleep(Duration::from_secs(2)).await;

        let test_deleted = store.delete_expired(Some(batch_size)).await.unwrap();
        assert!(
            test_deleted <= 2,
            "Batch size not respected in test deletion: {}",
            test_deleted
        );
    }

    // Continue deleting until no more expired sessions remain
    let mut total_iterations = 0;
    loop {
        let deleted = store.delete_expired(Some(batch_size)).await.unwrap();
        if deleted == 0 {
            break;
        }
        // Ensure batch size is respected
        assert!(
            deleted <= 2,
            "Batch size not respected: deleted {}",
            deleted
        );
        total_iterations += 1;
        // Safety check to prevent infinite loop
        assert!(total_iterations < 10, "Too many iterations");
    }
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

    // Verify all large data is preserved correctly by comparing with original
    for (key, expected_value) in &state {
        assert_eq!(
            loaded_record.state.get(key).unwrap(),
            expected_value,
            "Mismatch for large data key: {}",
            key
        );
    }

    // Verify we have the same number of keys
    assert_eq!(loaded_record.state.len(), state.len());
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

    // Verify all special characters and unicode are preserved by comparing with original
    for (key, expected_value) in &state {
        assert_eq!(
            loaded_record.state.get(key).unwrap(),
            expected_value,
            "Mismatch for unicode/special char key: {}",
            key
        );
    }

    // Verify we have the same number of keys
    assert_eq!(loaded_record.state.len(), state.len());
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

    // Spawn multiple concurrent operations on the same store
    // The underlying connection pool will handle concurrent access
    let id1 = session_id.clone();
    let id2 = session_id.clone();
    let id3 = session_id.clone();

    let (result1, result2, result3) = tokio::join!(
        store.load(&id1),
        store.update_ttl(&id2, Duration::from_secs(7200)),
        store.load(&id3)
    );

    // All operations should succeed
    assert!(result1.unwrap().is_some());
    assert!(result2.is_ok());
    assert!(result3.unwrap().is_some());
}

// Unhappy path tests

#[tokio::test]
async fn test_create_with_duplicate_id() {
    let store = create_test_store().await;
    let (session_id, state) = create_test_record(3600);

    let record = SessionRecordRef {
        state: Cow::Borrowed(&state),
        ttl: Duration::from_secs(3600),
    };

    // Create initial session
    store.create(&session_id, record).await.unwrap();

    // New state that will overwrite the original one
    // Each field is different.
    let mut new_state = HashMap::new();
    new_state.insert(
        Cow::Borrowed("user_id"),
        serde_json::Value::String("different-user-id".to_string()),
    );
    new_state.insert(
        Cow::Borrowed("login_time"),
        serde_json::Value::String("2024-02-01T00:00:00Z".to_string()),
    );
    new_state.insert(
        Cow::Borrowed("counter"),
        serde_json::Value::Number(50.into()),
    );
    new_state.insert(
        Cow::Borrowed("theme"),
        serde_json::Value::String("light".to_string()),
    );

    let conflicting_record = SessionRecordRef {
        state: Cow::Borrowed(&new_state),
        ttl: Duration::from_secs(7200),
    };

    // This should succeed due to ON DUPLICATE KEY UPDATE clause
    // We're in fact performing an upsert.
    store.create(&session_id, conflicting_record).await.unwrap();

    // Original data should be overwritten
    let loaded_after = store.load(&session_id).await.unwrap().unwrap();
    for (key, expected_value) in &new_state {
        assert_eq!(
            loaded_after.state.get(key).unwrap(),
            expected_value,
            "Original data should be overwritten when session exists"
        );
    }
}

#[tokio::test]
async fn test_update_unknown_id_error() {
    let store = create_test_store().await;
    let non_existent_id = SessionId::random();
    let (_, state) = create_test_record(3600);

    let record = SessionRecordRef {
        state: Cow::Borrowed(&state),
        ttl: Duration::from_secs(3600),
    };

    // Try to update a session that doesn't exist
    let result = store.update(&non_existent_id, record).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        pavex_session::store::errors::UpdateError::UnknownIdError(err) => {
            assert!(err.id == non_existent_id);
        }
        other => panic!("Expected UnknownId error, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_update_ttl_unknown_id_error() {
    let store = create_test_store().await;
    let non_existent_id = SessionId::random();

    // Try to update TTL for a session that doesn't exist
    let result = store
        .update_ttl(&non_existent_id, Duration::from_secs(7200))
        .await;

    assert!(result.is_err());
    match result.unwrap_err() {
        pavex_session::store::errors::UpdateTtlError::UnknownId(err) => {
            assert!(err.id == non_existent_id);
        }
        other => panic!("Expected UnknownId error, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_delete_unknown_id_error() {
    let store = create_test_store().await;
    let non_existent_id = SessionId::random();

    // Try to delete a session that doesn't exist
    let result = store.delete(&non_existent_id).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        pavex_session::store::errors::DeleteError::UnknownId(err) => {
            assert!(err.id == non_existent_id);
        }
        other => panic!("Expected UnknownId error, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_change_id_unknown_old_id_error() {
    let store = create_test_store().await;
    let non_existent_old_id = SessionId::random();
    let new_id = SessionId::random();

    // Try to change ID for a session that doesn't exist
    let result = store.change_id(&non_existent_old_id, &new_id).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        pavex_session::store::errors::ChangeIdError::UnknownId(err) => {
            assert!(err.id == non_existent_old_id);
        }
        other => panic!("Expected UnknownId error, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_change_id_duplicate_new_id_error() {
    let store = create_test_store().await;
    let (session_id_1, state_1) = create_test_record(3600);
    let (session_id_2, state_2) = create_test_record(3600);

    // Create two different sessions
    let record_1 = SessionRecordRef {
        state: Cow::Borrowed(&state_1),
        ttl: Duration::from_secs(3600),
    };
    let record_2 = SessionRecordRef {
        state: Cow::Borrowed(&state_2),
        ttl: Duration::from_secs(3600),
    };

    store.create(&session_id_1, record_1).await.unwrap();
    store.create(&session_id_2, record_2).await.unwrap();

    // Try to change session_id_1 to session_id_2 (which already exists)
    let result = store.change_id(&session_id_1, &session_id_2).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        pavex_session::store::errors::ChangeIdError::DuplicateId(err) => {
            assert!(err.id == session_id_2);
        }
        other => panic!("Expected DuplicateId error, got: {:?}", other),
    }

    // Verify both original sessions still exist
    assert!(store.load(&session_id_1).await.unwrap().is_some());
    assert!(store.load(&session_id_2).await.unwrap().is_some());
}

#[tokio::test]
async fn test_operations_on_expired_session() {
    let store = create_test_store().await;
    let (session_id, state) = create_test_record(1);

    let record = SessionRecordRef {
        state: Cow::Borrowed(&state),
        ttl: Duration::from_secs(1), // Very short TTL
    };

    // Create session with short TTL
    store.create(&session_id, record).await.unwrap();

    // Wait for expiration
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Try to update expired session - should return UnknownId error
    let (_, new_state) = create_test_record(3600);
    let new_record = SessionRecordRef {
        state: Cow::Borrowed(&new_state),
        ttl: Duration::from_secs(3600),
    };

    let update_result = store.update(&session_id, new_record).await;
    assert!(update_result.is_err());
    match update_result.unwrap_err() {
        pavex_session::store::errors::UpdateError::UnknownIdError(err) => {
            assert!(err.id == session_id);
        }
        other => panic!(
            "Expected UnknownId error for expired session update, got: {:?}",
            other
        ),
    }

    // Try to update TTL of expired session - should return UnknownId error
    let update_ttl_result = store
        .update_ttl(&session_id, Duration::from_secs(7200))
        .await;
    assert!(update_ttl_result.is_err());
    match update_ttl_result.unwrap_err() {
        pavex_session::store::errors::UpdateTtlError::UnknownId(err) => {
            assert!(err.id == session_id);
        }
        other => panic!(
            "Expected UnknownId error for expired session TTL update, got: {:?}",
            other
        ),
    }

    // Try to delete expired session - should return UnknownId error
    let delete_result = store.delete(&session_id).await;
    assert!(delete_result.is_err());
    match delete_result.unwrap_err() {
        pavex_session::store::errors::DeleteError::UnknownId(err) => {
            assert!(err.id == session_id);
        }
        other => panic!(
            "Expected UnknownId error for expired session delete, got: {:?}",
            other
        ),
    }

    // Try to change ID of expired session - should return UnknownId error
    let new_id = SessionId::random();
    let change_id_result = store.change_id(&session_id, &new_id).await;
    assert!(change_id_result.is_err());
    match change_id_result.unwrap_err() {
        pavex_session::store::errors::ChangeIdError::UnknownId(err) => {
            assert!(err.id == session_id);
        }
        other => panic!(
            "Expected UnknownId error for expired session ID change, got: {:?}",
            other
        ),
    }
}

#[tokio::test]
async fn test_serialization_error() {
    let store = create_test_store().await;
    let session_id = SessionId::random();

    // Create a problematic state that might cause serialization issues
    let mut state = HashMap::new();

    // JSON serialization should handle this fine, but let's test with some edge cases
    state.insert(Cow::Borrowed("inf_value"), serde_json::json!(f64::INFINITY));

    let record = SessionRecordRef {
        state: Cow::Borrowed(&state),
        ttl: Duration::from_secs(3600),
    };

    // This should succeed because serde_json handles infinity as null in JSON
    let result = store.create(&session_id, record).await;

    // If it fails, it should be a serialization error
    match result {
        Ok(_) => {
            // Verify we can load it back
            let loaded = store.load(&session_id).await.unwrap().unwrap();
            // Infinity becomes null in JSON
            assert!(loaded.state.get("inf_value").unwrap().is_null());
        }
        Err(pavex_session::store::errors::CreateError::SerializationError(_)) => {
            // This is also acceptable - serialization failed as expected
        }
        Err(other) => panic!("Unexpected error type: {:?}", other),
    }
}

#[tokio::test]
async fn test_database_unavailable_error() {
    // Create a store with an invalid connection to simulate database unavailability
    let invalid_url = "mysql://invalid_user:invalid_password@localhost:19999/nonexistent_db";

    // Try to connect to invalid database - this should fail
    if MySqlConnection::connect(invalid_url).await.is_ok() {
        // If somehow this succeeds, skip this test
        println!("Warning: Expected database connection to fail, but it succeeded");
        return;
    }

    let pool = MySqlPoolOptions::new()
        .acquire_timeout(Duration::from_secs(2))
        .connect_lazy(invalid_url)
        .unwrap();

    let store = MySqlSessionStore::new(pool);
    let (session_id, state) = create_test_record(3600);

    // Operations should fail with database errors due to closed pool
    let create_result = store
        .create(
            &session_id,
            SessionRecordRef {
                state: Cow::Borrowed(&state),
                ttl: Duration::from_secs(3600),
            },
        )
        .await;
    match create_result.unwrap_err() {
        pavex_session::store::errors::CreateError::Other(_) => {
            // Expected - database connection error
        }
        other => panic!(
            "Expected Other error for database unavailability, got: {:?}",
            other
        ),
    }

    let load_result = store.load(&session_id).await;
    match load_result.unwrap_err() {
        pavex_session::store::errors::LoadError::Other(_) => {
            // Expected - database connection error
        }
        other => panic!(
            "Expected Other error for database unavailability, got: {:?}",
            other
        ),
    }
}
