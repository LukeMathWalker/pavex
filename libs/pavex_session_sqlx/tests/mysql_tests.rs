use pavex_session::SessionId;
use pavex_session::store::{SessionRecordRef, SessionStorageBackend};
use pavex_session_sqlx::MySqlSessionStore;
use serde_json;
use sqlx::MySqlPool;
use std::borrow::Cow;
use std::collections::HashMap;
use std::num::NonZeroUsize;

use std::time::Duration;

async fn create_test_store() -> MySqlSessionStore {
    let database_url = std::env::var("TEST_MYSQL_URL")
        .unwrap_or_else(|_| "mysql://root:password@localhost:3306/test_sessions".to_string());

    let pool = MySqlPool::connect(&database_url)
        .await
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

    // Verify the data matches
    assert_eq!(loaded_record.state, state);
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

    // Load and verify
    let loaded_record = store.load(&session_id).await.unwrap().unwrap();
    assert_eq!(loaded_record.state, updated_state);
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

    // Load and verify TTL
    let loaded_record = store.load(&session_id).await.unwrap().unwrap();
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
    assert_eq!(loaded_record.state, state);
}

#[tokio::test]
async fn test_delete_expired() {
    let store = create_test_store().await;

    // Create expired sessions by directly inserting with past deadlines
    use pavex::time::Timestamp;
    let current_time = Timestamp::now().as_second();
    let past_deadline = current_time - 3600; // 1 hour ago

    let mut expired_session_ids = Vec::new();
    for i in 0..5 {
        let session_id = SessionId::random();
        expired_session_ids.push(session_id.clone());
        let state = serde_json::json!({
            "session_name": format!("expired_session_{}", i)
        });

        // Insert directly with past deadline
        sqlx::query("INSERT INTO sessions (id, deadline, state) VALUES (?, ?, ?)")
            .bind(session_id.inner().to_string())
            .bind(past_deadline)
            .bind(state)
            .execute(&store.0)
            .await
            .unwrap();
    }

    // Create a non-expired session normally
    let valid_session_id = SessionId::random();
    let mut valid_state = HashMap::new();
    valid_state.insert(
        Cow::Borrowed("session_name"),
        serde_json::Value::String("valid_session".to_string()),
    );
    let valid_record = SessionRecordRef {
        state: Cow::Borrowed(&valid_state),
        ttl: Duration::from_secs(3600),
    };
    store.create(&valid_session_id, valid_record).await.unwrap();

    // Verify expired sessions can't be loaded
    for session_id in &expired_session_ids {
        assert!(store.load(session_id).await.unwrap().is_none());
    }

    // Delete expired sessions - we don't check the exact count since other tests may have created expired sessions
    let deleted_count = store.delete_expired(None).await.unwrap();
    // We should have deleted at least our 5 expired sessions
    assert!(
        deleted_count >= 5,
        "Expected to delete at least 5 sessions, but deleted {}",
        deleted_count
    );

    // Verify our specific expired sessions are gone by trying to delete them again
    for session_id in &expired_session_ids {
        assert!(store.load(session_id).await.unwrap().is_none());
    }

    // Valid session should still exist
    assert!(store.load(&valid_session_id).await.unwrap().is_some());
}

#[tokio::test]
async fn test_delete_expired_with_batch_size() {
    let store = create_test_store().await;

    // Create expired sessions by directly inserting with past deadlines
    use pavex::time::Timestamp;
    let current_time = Timestamp::now().as_second();
    let past_deadline = current_time - 3600; // 1 hour ago

    let mut expired_session_ids = Vec::new();
    for i in 0..10 {
        let session_id = SessionId::random();
        expired_session_ids.push(session_id.clone());
        let state = serde_json::json!({
            "session_name": format!("expired_session_{}", i)
        });

        // Insert directly with past deadline
        sqlx::query("INSERT INTO sessions (id, deadline, state) VALUES (?, ?, ?)")
            .bind(session_id.inner().to_string())
            .bind(past_deadline)
            .bind(state)
            .execute(&store.0)
            .await
            .unwrap();
    }

    // Verify all sessions are expired
    for session_id in &expired_session_ids {
        assert!(store.load(session_id).await.unwrap().is_none());
    }

    // Test batch deletion functionality by calling it several times
    let batch_size = NonZeroUsize::new(3).unwrap();

    // First batch - should delete something (at least 1, up to 3)
    let first_batch = store.delete_expired(Some(batch_size)).await.unwrap();
    assert!(first_batch <= 3, "Batch size should be respected");

    // Continue deleting until all our sessions are gone
    let mut iterations = 0;
    let mut all_sessions_deleted = false;

    while !all_sessions_deleted && iterations < 10 {
        store.delete_expired(Some(batch_size)).await.unwrap();

        // Check if all our expired sessions are gone
        all_sessions_deleted = true;
        for session_id in &expired_session_ids {
            if store.load(session_id).await.unwrap().is_some() {
                all_sessions_deleted = false;
                break;
            }
        }
        iterations += 1;
    }

    // Verify our specific expired sessions are gone
    for session_id in &expired_session_ids {
        assert!(store.load(session_id).await.unwrap().is_none());
    }

    // Final call should return 0 (no more expired sessions to delete)
    let _final_batch = store.delete_expired(Some(batch_size)).await.unwrap();
    // Note: We can't assert this is exactly 0 because other tests might create expired sessions
    // but we can verify the functionality worked for our sessions
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
    assert_eq!(loaded_record.state, state);
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
    assert_eq!(loaded_record.state, state);
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

    // Spawn multiple concurrent operations
    let store_clone1 = store.clone();
    let store_clone2 = store.clone();
    let store_clone3 = store.clone();
    let id1 = session_id.clone();
    let id2 = session_id.clone();
    let id3 = session_id.clone();

    let (result1, result2, result3) = tokio::join!(
        store_clone1.load(&id1),
        store_clone2.update_ttl(&id2, Duration::from_secs(7200)),
        store_clone3.load(&id3)
    );

    // All operations should succeed
    assert!(result1.unwrap().is_some());
    assert!(result2.is_ok());
    assert!(result3.unwrap().is_some());
}
