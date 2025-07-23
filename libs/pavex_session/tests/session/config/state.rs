use pavex_session::{Session, SessionConfig, config::TtlExtensionThreshold};

use crate::fixtures::spy_store;

#[test]
fn test_ttl_extension_threshold_valid() {
    let valid_values = [0.0, 0.5, 1.0];
    for &value in &valid_values {
        let threshold = TtlExtensionThreshold::new(value);
        assert!(threshold.is_ok(), "Expected value {value} to be valid");
    }
}

#[test]
fn test_ttl_extension_threshold_invalid() {
    let invalid_values = [-0.1, 1.1];
    for &value in &invalid_values {
        let threshold = TtlExtensionThreshold::new(value);
        assert!(threshold.is_err(), "Expected value {value} to be invalid");
    }
}

#[test]
fn test_ttl_extension_threshold_try_from_f32() {
    let valid_value: f32 = 0.5;
    let invalid_value: f32 = 1.5;

    let valid_threshold = TtlExtensionThreshold::try_from(valid_value);
    assert!(
        valid_threshold.is_ok(),
        "Expected value {valid_value} to be valid"
    );

    let invalid_threshold = TtlExtensionThreshold::try_from(invalid_value);
    assert!(
        invalid_threshold.is_err(),
        "Expected value {invalid_value} to be invalid"
    );
}

#[test]
fn test_ttl_extension_threshold_try_from_f64() {
    let valid_value: f64 = 0.5;
    let invalid_value: f64 = 1.5;

    let valid_threshold = TtlExtensionThreshold::try_from(valid_value);
    assert!(
        valid_threshold.is_ok(),
        "Expected value {valid_value} to be valid"
    );

    let invalid_threshold = TtlExtensionThreshold::try_from(invalid_value);
    assert!(
        invalid_threshold.is_err(),
        "Expected value {invalid_value} to be invalid"
    );
}

#[test]
fn test_ttl_extension_threshold_deserialization() {
    let valid_threshold: Result<TtlExtensionThreshold, _> = serde_json::from_str("0.5");
    assert!(valid_threshold.is_ok(), "Expected JSON value to be valid");

    let invalid_threshold: Result<TtlExtensionThreshold, _> = serde_json::from_str("1.5");
    assert!(
        invalid_threshold.is_err(),
        "Expected JSON value to be invalid"
    );
}

#[tokio::test]
async fn default_ttl_can_be_changed() {
    let ((store, call_tracker), mut config) = (spy_store(), SessionConfig::default());
    let ttl_seconds = 10;
    config.state.ttl = std::time::Duration::from_secs(ttl_seconds);

    let mut session = Session::new(&store, &config, None);
    session.insert("key", "value").await.unwrap();
    session.finalize().await.unwrap().unwrap();

    let oplog = call_tracker.operation_log().await;
    let last = oplog.last().unwrap();
    assert_eq!(last, &format!("create <id> {ttl_seconds}s"));
}

#[tokio::test]
async fn default_ttl() {
    let ((store, call_tracker), config) = (spy_store(), SessionConfig::default());

    let mut session = Session::new(&store, &config, None);
    session.insert("key", "value").await.unwrap();
    session.finalize().await.unwrap().unwrap();

    let oplog = call_tracker.operation_log().await;
    let last = oplog.last().unwrap();
    assert_eq!(last, &format!("create <id> {}s", 60 * 60 * 24));
}
