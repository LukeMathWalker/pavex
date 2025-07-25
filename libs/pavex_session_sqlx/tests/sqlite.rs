use pavex_session::SessionId;
use pavex_session::store::{SessionRecordRef, SessionStorageBackend};
use pavex_session_sqlx::SqliteSessionStore;
use sqlx::SqlitePool;
use std::borrow::Cow;
use std::collections::HashMap;
use std::time::Duration;

async fn create_test_store() -> SqliteSessionStore {
    let database_url = "sqlite::memory:";
    let pool = SqlitePool::connect(database_url).await.unwrap();
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
    let database_url = "sqlite::memory:";
    let pool = SqlitePool::connect(database_url).await.unwrap();
    let store = SqliteSessionStore::new(pool);

    // Run migration multiple times - should not fail
    store.migrate().await.unwrap();
    store.migrate().await.unwrap();
    store.migrate().await.unwrap();

    // Create a test session to verify migration worked
    let (session_id, state) = create_test_record(3600);
    let record = SessionRecordRef {
        state: Cow::Borrowed(&state),
        ttl: Duration::from_secs(3600),
    };

    // If this succeeds, the migration worked properly
    store.create(&session_id, record).await.unwrap();
    let loaded = store.load(&session_id).await.unwrap();
    assert!(loaded.is_some());
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

    // Verify all data is preserved correctly by comparing with original
    for (key, expected_value) in &state {
        assert_eq!(
            loaded_record.state.get(key).unwrap(),
            expected_value,
            "Mismatch for key: {key}"
        );
    }

    // Verify we have the same number of keys
    assert_eq!(loaded_record.state.len(), state.len());

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

    // Verify all updated data is preserved correctly by comparing with updated state
    for (key, expected_value) in &state {
        assert_eq!(
            loaded.state.get(key).unwrap(),
            expected_value,
            "Mismatch for updated key: {key}"
        );
    }

    // Verify we have the same number of keys
    assert_eq!(loaded.state.len(), state.len());

    // Verify TTL was updated
    assert!(loaded.ttl.as_secs() > 3600);
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

    // Verify original data is preserved by comparing with original state
    for (key, expected_value) in &state {
        assert_eq!(
            loaded.state.get(key).unwrap(),
            expected_value,
            "Mismatch for key after TTL update: {key}"
        );
    }
    assert!(loaded.ttl.as_secs() > 3600);
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

    // Verify all data was transferred to new session ID
    for (key, expected_value) in &state {
        assert_eq!(
            new_record.state.get(key).unwrap(),
            expected_value,
            "Mismatch for key after ID change: {key}"
        );
    }
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

    // Verify all large data is preserved correctly by comparing with original
    for (key, expected_value) in &state {
        assert_eq!(
            loaded.state.get(key).unwrap(),
            expected_value,
            "Mismatch for large data key: {key}"
        );
    }

    // Verify we have the same number of keys
    assert_eq!(loaded.state.len(), state.len());
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

    // Verify all special characters and unicode are preserved by comparing with original
    for (key, expected_value) in &state {
        assert_eq!(
            loaded.state.get(key).unwrap(),
            expected_value,
            "Mismatch for unicode/special char key: {key}"
        );
    }

    // Verify we have the same number of keys
    assert_eq!(loaded.state.len(), state.len());
}

#[tokio::test]
async fn test_concurrent_operations() {
    // Create a shared database pool for all concurrent operations
    let database_url = "sqlite::memory:";
    let pool = SqlitePool::connect(database_url).await.unwrap();
    let store = SqliteSessionStore::new(pool.clone());
    store.migrate().await.unwrap();

    let mut handles = vec![];

    // Create multiple concurrent sessions using the same pool
    for i in 0..10 {
        let pool_clone = pool.clone();
        let handle = tokio::spawn(async move {
            let store_clone = SqliteSessionStore::new(pool_clone);
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

            // Verify we can load it back and all data is preserved
            let loaded = store_clone.load(&session_id).await.unwrap().unwrap();

            // Compare against the modified state we created
            for (key, expected_value) in &modified_state {
                assert_eq!(
                    loaded.state.get(key).unwrap(),
                    expected_value,
                    "Mismatch for key {key} in concurrent operation {i}"
                );
            }

            session_id
        });
        handles.push(handle);
    }

    // Wait for all operations to complete
    let mut session_ids = Vec::new();
    for handle in handles {
        session_ids.push(handle.await.unwrap());
    }

    // Verify all sessions exist using the shared store
    for session_id in session_ids {
        let loaded = store.load(&session_id).await.unwrap();
        assert!(loaded.is_some());
    }
}

// Unhappy path tests - Error scenarios

#[tokio::test]
async fn test_create_duplicate_id_error() {
    let store = create_test_store().await;
    let (session_id, state) = create_test_record(3600);

    let record = SessionRecordRef {
        state: Cow::Borrowed(&state),
        ttl: Duration::from_secs(3600),
    };

    // Create initial session
    store.create(&session_id, record).await.unwrap();

    // Try to create another session with the same ID but different data
    let (_, different_state) = create_test_record(7200);
    let mut conflicting_state = different_state;
    conflicting_state.insert(
        Cow::Borrowed("conflict_field"),
        serde_json::Value::String("this should conflict".to_string()),
    );

    let conflicting_record = SessionRecordRef {
        state: Cow::Borrowed(&conflicting_state),
        ttl: Duration::from_secs(1), // Short TTL to force conflict
    };

    // This should succeed due to ON CONFLICT clause with deadline check
    // But if we modify the query to not have ON CONFLICT, it would be a duplicate error
    // For now, let's test the case where the session exists and is not expired

    // First, let's verify the original session exists
    let loaded = store.load(&session_id).await.unwrap();
    assert!(loaded.is_some());

    // The ON CONFLICT clause in our implementation means this won't error,
    // but it will only update if the existing session is expired
    // Since our session isn't expired, the new data won't be written
    store.create(&session_id, conflicting_record).await.unwrap();

    // Verify the original data is still there (not overwritten)
    let loaded_after = store.load(&session_id).await.unwrap().unwrap();
    for (key, expected_value) in &state {
        assert_eq!(
            loaded_after.state.get(key).unwrap(),
            expected_value,
            "Original data should be preserved when session is not expired"
        );
    }

    // Verify conflicting data was not written
    assert!(loaded_after.state.get("conflict_field").is_none());
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
        other => panic!("Expected UnknownId error, got: {other:?}"),
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
        other => panic!("Expected UnknownId error, got: {other:?}"),
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
        other => panic!("Expected UnknownId error, got: {other:?}"),
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
        other => panic!("Expected UnknownId error, got: {other:?}"),
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
        other => panic!("Expected DuplicateId error, got: {other:?}"),
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
        other => panic!("Expected UnknownId error for expired session update, got: {other:?}"),
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
        other => panic!("Expected UnknownId error for expired session TTL update, got: {other:?}"),
    }

    // Try to delete expired session - should return UnknownId error
    let delete_result = store.delete(&session_id).await;
    assert!(delete_result.is_err());
    match delete_result.unwrap_err() {
        pavex_session::store::errors::DeleteError::UnknownId(err) => {
            assert!(err.id == session_id);
        }
        other => panic!("Expected UnknownId error for expired session delete, got: {other:?}"),
    }

    // Try to change ID of expired session - should return UnknownId error
    let new_id = SessionId::random();
    let change_id_result = store.change_id(&session_id, &new_id).await;
    assert!(change_id_result.is_err());
    match change_id_result.unwrap_err() {
        pavex_session::store::errors::ChangeIdError::UnknownId(err) => {
            assert!(err.id == session_id);
        }
        other => panic!("Expected UnknownId error for expired session ID change, got: {other:?}"),
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
        Err(other) => panic!("Unexpected error type: {other:?}"),
    }
}

#[tokio::test]
async fn test_database_unavailable_error() {
    // Create a store with a closed pool to simulate database unavailability
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    let store = SqliteSessionStore::new(pool.clone());
    store.migrate().await.unwrap();

    // Close the pool
    pool.close().await;

    let (session_id, state) = create_test_record(3600);
    let record = SessionRecordRef {
        state: Cow::Borrowed(&state),
        ttl: Duration::from_secs(3600),
    };

    // Operations should fail with database errors
    let create_result = store.create(&session_id, record).await;
    assert!(create_result.is_err());
    match create_result.unwrap_err() {
        pavex_session::store::errors::CreateError::Other(_) => {
            // Expected - database connection error
        }
        other => panic!("Expected Other error for database unavailability, got: {other:?}"),
    }

    let load_result = store.load(&session_id).await;
    assert!(load_result.is_err());
    match load_result.unwrap_err() {
        pavex_session::store::errors::LoadError::Other(_) => {
            // Expected - database connection error
        }
        other => panic!("Expected Other error for database unavailability, got: {other:?}"),
    }
}
