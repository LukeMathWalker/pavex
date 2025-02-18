use pavex_session::config::TtlExtensionThreshold;

#[test]
fn test_ttl_extension_threshold_valid() {
    let valid_values = [0.0, 0.5, 1.0];
    for &value in &valid_values {
        let threshold = TtlExtensionThreshold::new(value);
        assert!(threshold.is_ok(), "Expected value {} to be valid", value);
    }
}

#[test]
fn test_ttl_extension_threshold_invalid() {
    let invalid_values = [-0.1, 1.1];
    for &value in &invalid_values {
        let threshold = TtlExtensionThreshold::new(value);
        assert!(threshold.is_err(), "Expected value {} to be invalid", value);
    }
}

#[test]
fn test_ttl_extension_threshold_try_from_f32() {
    let valid_value: f32 = 0.5;
    let invalid_value: f32 = 1.5;

    let valid_threshold = TtlExtensionThreshold::try_from(valid_value);
    assert!(
        valid_threshold.is_ok(),
        "Expected value {} to be valid",
        valid_value
    );

    let invalid_threshold = TtlExtensionThreshold::try_from(invalid_value);
    assert!(
        invalid_threshold.is_err(),
        "Expected value {} to be invalid",
        invalid_value
    );
}

#[test]
fn test_ttl_extension_threshold_try_from_f64() {
    let valid_value: f64 = 0.5;
    let invalid_value: f64 = 1.5;

    let valid_threshold = TtlExtensionThreshold::try_from(valid_value);
    assert!(
        valid_threshold.is_ok(),
        "Expected value {} to be valid",
        valid_value
    );

    let invalid_threshold = TtlExtensionThreshold::try_from(invalid_value);
    assert!(
        invalid_threshold.is_err(),
        "Expected value {} to be invalid",
        invalid_value
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
