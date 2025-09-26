/// Tests for crypto provider feature handling

#[test]
#[cfg(not(feature = "tls_crypto_provider_aws_lc_rs"))]
fn aws_lc_rs_requires_feature() {
    use pavex::tls::client::TlsClientPolicyConfig;

    let yaml = r#"
crypto_provider:
  name: aws-lc-rs
"#;
    let config: TlsClientPolicyConfig = serde_yaml::from_str(yaml).unwrap();
    let err = config.rustls_0_23_config().unwrap_err();

    insta::assert_snapshot!(err, @r###"
    Your TLS client configuration wants to use `aws_lc_rs` as its cryptography stack, but the corresponding `cargo` feature is not enabled.
    Add `tls_crypto_provider_aws_lc_rs` to the `features` array for `pavex` in your Cargo.toml manifest.
    "###);
}

#[test]
#[cfg(not(feature = "tls_crypto_provider_ring"))]
fn ring_requires_feature() {
    use pavex::tls::client::TlsClientPolicyConfig;

    let yaml = r#"
crypto_provider:
  name: ring
"#;
    let config: TlsClientPolicyConfig = serde_yaml::from_str(yaml).unwrap();
    let err = config.rustls_0_23_config().unwrap_err();

    insta::assert_snapshot!(err, @r###"
    Your TLS client configuration wants to use `ring` as its cryptography stack, but the corresponding `cargo` feature is not enabled.
    Add `tls_crypto_provider_ring` to the `features` array for `pavex` in your Cargo.toml manifest.
    "###);
}

#[test]
#[cfg(feature = "tls_crypto_provider_aws_lc_rs")]
fn aws_lc_rs_works_when_feature_enabled() {
    use pavex::tls::client::TlsClientPolicyConfig;

    let yaml = r#"
crypto_provider:
  name: aws-lc-rs
"#;
    let config: TlsClientPolicyConfig = serde_yaml::from_str(yaml).unwrap();
    // Should not error when feature is enabled
    config.rustls_0_23_config().unwrap();
}

#[test]
#[cfg(feature = "tls_crypto_provider_ring")]
fn ring_works_when_feature_enabled() {
    use pavex::tls::client::TlsClientPolicyConfig;

    let yaml = r#"
crypto_provider:
  name: ring
"#;
    let config: TlsClientPolicyConfig = serde_yaml::from_str(yaml).unwrap();
    // Should not error when feature is enabled
    config.rustls_0_23_config().unwrap();
}

#[test]
#[cfg(all(feature = "tls_crypto_provider_aws_lc_rs", not(feature = "fips")))]
fn fips_required_but_not_enabled() {
    use pavex::tls::client::TlsClientPolicyConfig;

    let yaml = r#"
crypto_provider:
  name: aws-lc-rs
  require_fips: true
"#;
    let config: TlsClientPolicyConfig = serde_yaml::from_str(yaml).unwrap();
    let err = config.rustls_0_23_config().unwrap_err();

    insta::assert_snapshot!(err, @"FIPS requested but the `fips` feature is not enabled. Add `fips` to the `features` array for `pavex` in your Cargo.toml manifest.");
}

#[test]
#[cfg(all(feature = "tls_crypto_provider_aws_lc_rs", feature = "fips"))]
fn fips_works_when_enabled() {
    use pavex::tls::client::TlsClientPolicyConfig;

    let yaml = r#"
crypto_provider:
  name: aws-lc-rs
  require_fips: true
"#;
    let config: TlsClientPolicyConfig = serde_yaml::from_str(yaml).unwrap();
    // Should not error when FIPS feature is enabled
    config.rustls_0_23_config().unwrap();
}
