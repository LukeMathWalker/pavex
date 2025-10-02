use pavex::tls::client::TlsClientPolicyConfig;

#[test]
fn both_tls_versions_disabled() {
    let yaml = r#"
allowed_versions:
  v1_2: false
  v1_3: false
"#;
    let config: TlsClientPolicyConfig = serde_yaml::from_str(yaml).unwrap();
    let err = config.rustls_0_23_config().unwrap_err();

    insta::assert_snapshot!(err, @"You disabled both TLS 1.2 and TLS 1.3. At least one of them must be enabled.");
}

#[test]
fn os_verifier_disabled_with_no_additional_roots() {
    let yaml = r#"
certificate_verification:
  use_os_verifier: false
  additional_roots: []
"#;
    let config: TlsClientPolicyConfig = serde_yaml::from_str(yaml).unwrap();
    let err = config.rustls_0_23_config().unwrap_err();

    insta::assert_snapshot!(err, @r###"
    You disabled OS server certificate verification without providing a list of additional root certificates to trust.
    This configuration is invalid: it would cause **all** TLS connections to fail.
    Please enable OS certificate verification or provide a list of root certificates to trust. Check out the documentation of `pavex::tls::client::CertificateVerificationConfig` for more information.
    "###);
}

#[test]
fn empty_pem_bundle() {
    let yaml = r#"
certificate_verification:
  additional_roots:
    - inline:
        encoding: pem
        data: |
          # Just a comment, no certificates
"#;
    let config: TlsClientPolicyConfig = serde_yaml::from_str(yaml).unwrap();
    let err = config.rustls_0_23_config().unwrap_err();

    insta::assert_snapshot!(format!("{:#}", err), @"Failed to parse the root certificate at index 0 in your list of `additional_roots`: Your PEM bundle doesn't contain any root certificate. There should be at least one `BEGIN CERTIFICATE` block");
}

#[test]
fn pem_with_wrong_section_type() {
    // This is a PRIVATE KEY, not a CERTIFICATE
    let yaml = r#"
certificate_verification:
  additional_roots:
    - inline:
        encoding: pem
        data: |
          -----BEGIN PRIVATE KEY-----
          MIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQC7VJTUt9Us8cKj
          -----END PRIVATE KEY-----
"#;
    let config: TlsClientPolicyConfig = serde_yaml::from_str(yaml).unwrap();
    let err = config.rustls_0_23_config().unwrap_err();

    insta::assert_snapshot!(format!("{:#}", err), @"Failed to parse the root certificate at index 0 in your list of `additional_roots`: Expected a PEM-encoded root certificate, but found a PRIVATE KEY section");
}

#[test]
fn malformed_pem() {
    let yaml = r#"
certificate_verification:
  additional_roots:
    - inline:
        encoding: pem
        data: |
          -----BEGIN CERTIFICATE-----
          This is not valid base64 PEM content
          -----END CERTIFICATE-----
"#;
    let config: TlsClientPolicyConfig = serde_yaml::from_str(yaml).unwrap();
    let err = config.rustls_0_23_config().unwrap_err();

    insta::assert_snapshot!(format!("{:#}", err), @"Failed to initialize the server certificate verifier: invalid peer certificate: BadEncoding");
}
