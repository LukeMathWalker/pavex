use pavex::tls::client::TlsClientPolicyConfig;
use pavex::tls::client::config::{
    CryptoProviderConfig, RootCertificate, RootCertificateFileEncoding,
    RootCertificateInlineEncoding,
};

/// Tests that verify the YAML examples from the documentation deserialize correctly.
mod doc_examples {
    use super::*;

    #[test]
    fn test_default_config() {
        let yaml = include_str!("../../fixtures/tls_config/default.yaml");
        let config: TlsClientPolicyConfig = serde_yaml::from_str(yaml).unwrap();

        // Verify the default config values
        assert!(matches!(
            config.crypto_provider,
            CryptoProviderConfig::AwsLcRs {
                require_fips: false
            }
        ));
        assert!(config.allowed_versions.v1_2);
        assert!(config.allowed_versions.v1_3);
        assert!(config.certificate_verification.use_os_verifier);
        assert!(config.certificate_verification.additional_roots.is_empty());
        assert!(!config.insecure.skip_verification);

        // Verify it matches what you get from an empty document
        let empty_config: TlsClientPolicyConfig = serde_yaml::from_str("").unwrap();
        assert_eq!(
            serde_yaml::to_string(&config).unwrap(),
            serde_yaml::to_string(&empty_config).unwrap(),
            "Default config should match empty document"
        );
    }

    #[test]
    fn test_disable_tls_1_2() {
        let yaml = include_str!("../../fixtures/tls_config/disable_tls_1_2.yaml");
        let config: TlsClientPolicyConfig = serde_yaml::from_str(yaml).unwrap();

        assert!(!config.allowed_versions.v1_2);
        assert!(config.allowed_versions.v1_3);
    }

    #[test]
    fn test_additional_roots() {
        let yaml = include_str!("../../fixtures/tls_config/additional_roots.yaml");
        let config: TlsClientPolicyConfig = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(config.certificate_verification.additional_roots.len(), 2);
    }

    #[test]
    fn test_skip_verification() {
        let yaml = include_str!("../../fixtures/tls_config/skip_verification.yaml");
        let config: TlsClientPolicyConfig = serde_yaml::from_str(yaml).unwrap();

        assert!(config.insecure.skip_verification);
    }
}

#[test]
fn test_empty_config_uses_defaults() {
    let yaml = "";
    let config: TlsClientPolicyConfig = serde_yaml::from_str(yaml).unwrap();

    assert!(matches!(
        config.crypto_provider,
        CryptoProviderConfig::AwsLcRs {
            require_fips: false
        }
    ));
    assert!(config.allowed_versions.v1_2);
    assert!(config.allowed_versions.v1_3);
    assert!(config.certificate_verification.use_os_verifier);
    assert!(config.certificate_verification.additional_roots.is_empty());
    assert!(!config.insecure.skip_verification);
}

#[test]
fn test_disable_tls_1_3() {
    let yaml = r#"
allowed_versions:
  v1_3: false
"#;
    let config: TlsClientPolicyConfig = serde_yaml::from_str(yaml).unwrap();

    assert!(config.allowed_versions.v1_2);
    assert!(!config.allowed_versions.v1_3);
}

#[test]
fn test_additional_root_from_file_pem() {
    let yaml = r#"
certificate_verification:
  additional_roots:
    - file:
        path: /path/to/certificate.pem
        encoding: pem
"#;
    let config: TlsClientPolicyConfig = serde_yaml::from_str(yaml).unwrap();

    assert_eq!(config.certificate_verification.additional_roots.len(), 1);
    match &config.certificate_verification.additional_roots[0] {
        RootCertificate::File { encoding, path } => {
            assert!(matches!(encoding, RootCertificateFileEncoding::Pem));
            assert_eq!(path.to_str().unwrap(), "/path/to/certificate.pem");
        }
        _ => panic!("Expected File variant"),
    }
}

#[test]
fn test_additional_root_from_file_der() {
    let yaml = r#"
certificate_verification:
  additional_roots:
    - file:
        path: /path/to/certificate.der
        encoding: der
"#;
    let config: TlsClientPolicyConfig = serde_yaml::from_str(yaml).unwrap();

    assert_eq!(config.certificate_verification.additional_roots.len(), 1);
    match &config.certificate_verification.additional_roots[0] {
        RootCertificate::File { encoding, path } => {
            assert!(matches!(encoding, RootCertificateFileEncoding::Der));
            assert_eq!(path.to_str().unwrap(), "/path/to/certificate.der");
        }
        _ => panic!("Expected File variant"),
    }
}

#[test]
fn test_inline_root_pem() {
    let yaml = r#"
certificate_verification:
  additional_roots:
    - inline:
        encoding: pem
        data: |
          -----BEGIN CERTIFICATE-----
          MIICUTCCAfugAwIBAgIBADANBgkqhkiG9w0BAQQFADBXMQswCQYDVQQGEwJDTjEL
          -----END CERTIFICATE-----
"#;
    let config: TlsClientPolicyConfig = serde_yaml::from_str(yaml).unwrap();

    assert_eq!(config.certificate_verification.additional_roots.len(), 1);
    match &config.certificate_verification.additional_roots[0] {
        RootCertificate::Inline { encoding, data } => {
            assert!(matches!(encoding, RootCertificateInlineEncoding::Pem));
            assert!(data.contains("BEGIN CERTIFICATE"));
        }
        _ => panic!("Expected Inline variant"),
    }
}

#[test]
fn test_inline_root_base64_der() {
    let yaml = r#"
certificate_verification:
  additional_roots:
    - inline:
        encoding: base64_der
        data: MIICUTCCAfugAwIBAgIBADANBgkqhkiG9w0BAQQFADBXMQswCQYDVQQGEwJDTjEL
"#;
    let config: TlsClientPolicyConfig = serde_yaml::from_str(yaml).unwrap();

    assert_eq!(config.certificate_verification.additional_roots.len(), 1);
    match &config.certificate_verification.additional_roots[0] {
        RootCertificate::Inline { encoding, data } => {
            assert!(matches!(encoding, RootCertificateInlineEncoding::Base64Der));
            assert_eq!(
                data,
                "MIICUTCCAfugAwIBAgIBADANBgkqhkiG9w0BAQQFADBXMQswCQYDVQQGEwJDTjEL"
            );
        }
        _ => panic!("Expected Inline variant"),
    }
}

#[test]
fn test_multiple_additional_roots() {
    let yaml = r#"
certificate_verification:
  additional_roots:
    - file:
        path: /path/to/cert1.pem
        encoding: pem
    - file:
        path: /path/to/cert2.der
        encoding: "der"
    - inline:
        encoding: pem
        data: "-----BEGIN CERTIFICATE-----\ndata\n-----END CERTIFICATE-----"
"#;
    let config: TlsClientPolicyConfig = serde_yaml::from_str(yaml).unwrap();

    assert_eq!(config.certificate_verification.additional_roots.len(), 3);
}

#[test]
fn test_disable_os_verifier() {
    let yaml = r#"
certificate_verification:
  use_os_verifier: false
"#;
    let config: TlsClientPolicyConfig = serde_yaml::from_str(yaml).unwrap();

    assert!(!config.certificate_verification.use_os_verifier);
}

#[test]
fn test_skip_verification_enabled() {
    let yaml = r#"
insecure:
  skip_verification: true
"#;
    let config: TlsClientPolicyConfig = serde_yaml::from_str(yaml).unwrap();

    assert!(config.insecure.skip_verification);
}

#[test]
fn test_ring_crypto_provider() {
    let yaml = r#"
crypto_provider:
  name: ring
"#;
    let config: TlsClientPolicyConfig = serde_yaml::from_str(yaml).unwrap();

    assert!(matches!(config.crypto_provider, CryptoProviderConfig::Ring));
}

#[test]
fn test_aws_lc_rs_with_fips() {
    let yaml = r#"
crypto_provider:
  name: aws-lc-rs
  require_fips: true
"#;
    let config: TlsClientPolicyConfig = serde_yaml::from_str(yaml).unwrap();

    assert!(matches!(
        config.crypto_provider,
        CryptoProviderConfig::AwsLcRs { require_fips: true }
    ));
}

#[test]
fn test_complex_config() {
    let yaml = r#"
crypto_provider:
  name: aws-lc-rs
  require_fips: true
allowed_versions:
  v1_2: false
  v1_3: true
certificate_verification:
  use_os_verifier: true
  additional_roots:
    - file:
        path: /etc/ssl/certs/custom-ca.pem
        encoding: pem
insecure:
  skip_verification: false
"#;
    let config: TlsClientPolicyConfig = serde_yaml::from_str(yaml).unwrap();

    assert!(matches!(
        config.crypto_provider,
        CryptoProviderConfig::AwsLcRs { require_fips: true }
    ));
    assert!(!config.allowed_versions.v1_2);
    assert!(config.allowed_versions.v1_3);
    assert!(config.certificate_verification.use_os_verifier);
    assert_eq!(config.certificate_verification.additional_roots.len(), 1);
    assert!(!config.insecure.skip_verification);
}
