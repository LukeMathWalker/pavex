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
    let pool = SqlitePool::connect(&database_url).await.unwrap();
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

    // Verify all data is preserved correctly
    assert_eq!(
        loaded_record.state.get("user_id").unwrap(),
        &serde_json::Value::String("test-user-123".to_string())
    );
    assert_eq!(
        loaded_record.state.get("login_time").unwrap(),
        &serde_json::Value::String("2024-01-01T00:00:00Z".to_string())
    );
    assert_eq!(
        loaded_record.state.get("permissions").unwrap(),
        &serde_json::json!(["read", "write"])
    );

    // Verify nested JSONB structure
    let metadata = loaded_record.state.get("metadata").unwrap();
    assert_eq!(metadata.get("ip").unwrap(), "192.168.1.1");
    assert_eq!(metadata.get("user_agent").unwrap(), "test-agent");
    assert_eq!(metadata.get("session_start").unwrap(), 1640995200);

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

    assert_eq!(
        loaded.state.get("updated_field").unwrap(),
        &serde_json::Value::String("new_value".to_string())
    );
    assert_eq!(
        loaded.state.get("user_id").unwrap(),
        &serde_json::Value::String("updated-user-456".to_string())
    );

    // Verify complex nested structure is preserved
    let new_metadata = loaded.state.get("new_metadata").unwrap();
    assert_eq!(new_metadata.get("last_action").unwrap(), "update_session");
    assert_eq!(new_metadata.get("timestamp").unwrap(), 1640995260);

    let deeply_nested = &new_metadata["complex_data"]["nested"]["deeply"];
    assert_eq!(deeply_nested.as_array().unwrap().len(), 4);
    assert_eq!(deeply_nested[0], "nested");
    assert_eq!(deeply_nested[1], "array");
    assert_eq!(deeply_nested[2], 123);
    assert_eq!(deeply_nested[3], true);

    // Verify TTL was updated
    assert!(loaded.ttl.as_secs() > 7150);
    assert!(loaded.ttl.as_secs() <= 7200);
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
    assert_eq!(
        loaded.state.get("user_id").unwrap(),
        &serde_json::Value::String("test-user-123".to_string())
    );
    assert!(loaded.ttl.as_secs() > 7150);
    assert!(loaded.ttl.as_secs() <= 7200);
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
    assert_eq!(
        new_record.state.get("user_id").unwrap(),
        &serde_json::Value::String("test-user-123".to_string())
    );
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

    // Verify large string
    assert_eq!(
        loaded.state.get("large_string").unwrap(),
        &serde_json::Value::String(large_string)
    );

    // Verify large array
    let loaded_array = loaded.state.get("large_array").unwrap().as_array().unwrap();
    assert_eq!(loaded_array.len(), 1000);
    assert_eq!(loaded_array[0]["index"], 0);
    assert_eq!(loaded_array[999]["index"], 999);

    // Verify deeply nested structure
    let complex = &loaded.state["complex_object"]["level1"]["level2"]["level3"]["level4"];
    assert_eq!(complex["data"], "deeply nested");
    assert_eq!(complex["boolean"], true);
    assert!(complex["null_value"].is_null());
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

    // Verify all special characters and unicode are preserved
    assert_eq!(
        loaded.state.get("unicode").unwrap(),
        &serde_json::Value::String("Hello, 世界! 🌍 Здравствуй мир! 🎉".to_string())
    );
    assert_eq!(
        loaded.state.get("json_string").unwrap(),
        &serde_json::Value::String(r#"{"nested": "json", "quotes": "\"escaped\""}"#.to_string())
    );
    assert_eq!(
        loaded.state.get("special_chars").unwrap(),
        &serde_json::Value::String("Line1\nLine2\tTabbed\rCarriage\"Quoted\"".to_string())
    );

    let emoji_data = loaded.state.get("emoji_data").unwrap();
    assert_eq!(emoji_data["reactions"].as_array().unwrap().len(), 6);
    assert_eq!(emoji_data["reactions"][0], "👍");
    assert_eq!(
        emoji_data["message"],
        "Unicode test with émojis and àccénts"
    );
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

            // Verify we can load it back
            let loaded = store_clone.load(&session_id).await.unwrap().unwrap();
            assert_eq!(
                loaded.state.get("thread_id").unwrap(),
                &serde_json::Value::Number(i.into())
            );

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
