use pavex::tls::client::TlsClientPolicyConfig;

#[test]
fn default_config_builds_successfully() {
    let yaml = include_str!("../../../fixtures/tls_config/default.yaml");
    let config: TlsClientPolicyConfig = serde_yaml::from_str(yaml).unwrap();

    // Just verify it built successfully - we can't easily inspect internals
    config.rustls_0_23_config().unwrap();
}

#[test]
fn skip_verification_builds_successfully() {
    let yaml = include_str!("../../../fixtures/tls_config/skip_verification.yaml");
    let config: TlsClientPolicyConfig = serde_yaml::from_str(yaml).unwrap();

    config.rustls_0_23_config().unwrap();
}

#[test]
fn only_tls_1_2_builds_successfully() {
    let yaml = r#"
allowed_versions:
  v1_2: true
  v1_3: false
"#;
    let config: TlsClientPolicyConfig = serde_yaml::from_str(yaml).unwrap();

    config.rustls_0_23_config().unwrap();
}

#[test]
fn only_tls_1_3_builds_successfully() {
    let yaml = r#"
allowed_versions:
  v1_2: false
  v1_3: true
"#;
    let config: TlsClientPolicyConfig = serde_yaml::from_str(yaml).unwrap();

    config.rustls_0_23_config().unwrap();
}

#[test]
fn with_inline_pem_certificate_builds_successfully() {
    // Using a real (but expired) certificate for testing
    let yaml = r#"
certificate_verification:
  use_os_verifier: false
  additional_roots:
    - inline:
        encoding: pem
        data: |
          -----BEGIN CERTIFICATE-----
          MIIDQTCCAimgAwIBAgITBmyfz5m/jAo54vB4ikPmljZbyjANBgkqhkiG9w0BAQsF
          ADA5MQswCQYDVQQGEwJVUzEPMA0GA1UEChMGQW1hem9uMRkwFwYDVQQDExBBbWF6
          b24gUm9vdCBDQSAxMB4XDTE1MDUyNjAwMDAwMFoXDTM4MDExNzAwMDAwMFowOTEL
          MAkGA1UEBhMCVVMxDzANBgNVBAoTBkFtYXpvbjEZMBcGA1UEAxMQQW1hem9uIFJv
          b3QgQ0EgMTCCASIwDQYJKoZIhvcNAQEBBQADggEPADCCAQoCggEBALJ4gHHKeNXj
          ca9HgFB0fW7Y14h29Jlo91ghYPl0hAEvrAIthtOgQ3pOsqTQNroBvo3bSMgHFzZM
          9O6II8c+6zf1tRn4SWiw3te5djgdYZ6k/oI2peVKVuRF4fn9tBb6dNqcmzU5L/qw
          IFAGbHrQgLKm+a/sRxmPUDgH3KKHOVj4utWp+UhnMJbulHheb4mjUcAwhmahRWa6
          VOujw5H5SNz/0egwLX0tdHA114gk957EWW67c4cX8jJGKLhD+rcdqsq08p8kDi1L
          93FcXmn/6pUCyziKrlA4b9v7LWIbxcceVOF34GfID5yHI9Y/QCB/IIDEgEw+OyQm
          jgSubJrIqg0CAwEAAaNCMEAwDwYDVR0TAQH/BAUwAwEB/zAOBgNVHQ8BAf8EBAMC
          AYYwHQYDVR0OBBYEFIQYzIU07LwMlJQuCFmcx7IQTgoIMA0GCSqGSIb3DQEBCwUA
          A4IBAQCY8jdaQZChGsV2USggNiMOruYou6r4lK5IpDB/G/wkjUu0yKGX9rbxenDI
          U5PMCCjjmCXPI6T53iHTfIUJrU6adTrCC2qJeHZERxhlbI1Bjjt/msv0tadQ1wUs
          N+gDS63pYaACbvXy8MWy7Vu33PqUXHeeE6V/Uq2V8viTO96LXFvKWlJbYK8U90vv
          o/ufQJVtMVT8QtPHRh8jrdkPSHCa2XV4cdFyQzR1bldZwgJcJmApzyMZFo6IQ6XU
          5MsI+yMRQ+hDKXJioaldXgjUkK642M4UwtBV8ob2xJNDd2ZhwLnoQdeXeGADbkpy
          rqXRfboQnoZsG4q5WTP468SQvvG5
          -----END CERTIFICATE-----
"#;
    let config: TlsClientPolicyConfig = serde_yaml::from_str(yaml).unwrap();

    config.rustls_0_23_config().unwrap();
}

#[test]
fn with_inline_base64_der_certificate_builds_successfully() {
    // Same Amazon Root CA 1 certificate, but in base64 DER format
    let yaml = r#"
certificate_verification:
  use_os_verifier: false
  additional_roots:
    - inline:
        encoding: base64_der
        data: MIIDQTCCAimgAwIBAgITBmyfz5m/jAo54vB4ikPmljZbyjANBgkqhkiG9w0BAQsFADA5MQswCQYDVQQGEwJVUzEPMA0GA1UEChMGQW1hem9uMRkwFwYDVQQDExBBbWF6b24gUm9vdCBDQSAxMB4XDTE1MDUyNjAwMDAwMFoXDTM4MDExNzAwMDAwMFowOTELMAkGA1UEBhMCVVMxDzANBgNVBAoTBkFtYXpvbjEZMBcGA1UEAxMQQW1hem9uIFJvb3QgQ0EgMTCCASIwDQYJKoZIhvcNAQEBBQADggEPADCCAQoCggEBALJ4gHHKeNXjca9HgFB0fW7Y14h29Jlo91ghYPl0hAEvrAIthtOgQ3pOsqTQNroBvo3bSMgHFzZM9O6II8c+6zf1tRn4SWiw3te5djgdYZ6k/oI2peVKVuRF4fn9tBb6dNqcmzU5L/qwIFAGbHrQgLKm+a/sRxmPUDgH3KKHOVj4utWp+UhnMJbulHheb4mjUcAwhmahRWa6VOujw5H5SNz/0egwLX0tdHA114gk957EWW67c4cX8jJGKLhD+rcdqsq08p8kDi1L93FcXmn/6pUCyziKrlA4b9v7LWIbxcceVOF34GfID5yHI9Y/QCB/IIDEgEw+OyQmjgSubJrIqg0CAwEAAaNCMEAwDwYDVR0TAQH/BAUwAwEB/zAOBgNVHQ8BAf8EBAMCAYYwHQYDVR0OBBYEFIQYzIU07LwMlJQuCFmcx7IQTgoIMA0GCSqGSIb3DQEBCwUAA4IBAQCY8jdaQZChGsV2USggNiMOruYou6r4lK5IpDB/G/wkjUu0yKGX9rbxenDIU5PMCCjjmCXPI6T53iHTfIUJrU6adTrCC2qJeHZERxhlbI1Bjjt/msv0tadQ1wUsN+gDS63pYaACbvXy8MWy7Vu33PqUXHeeE6V/Uq2V8viTO96LXFvKWlJbYK8U90vvo/ufQJVtMVT8QtPHRh8jrdkPSHCa2XV4cdFyQzR1bldZwgJcJmApzyMZFo6IQ6XU5MsI+yMRQ+hDKXJioaldXgjUkK642M4UwtBV8ob2xJNDd2ZhwLnoQdeXeGADbkpyrqXRfboQnoZsG4q5WTP468SQvvG5
"#;
    let config: TlsClientPolicyConfig = serde_yaml::from_str(yaml).unwrap();

    config.rustls_0_23_config().unwrap();
}

#[test]
fn try_from_succeeds_for_valid_config() {
    let yaml = include_str!("../../../fixtures/tls_config/default.yaml");
    let config: TlsClientPolicyConfig = serde_yaml::from_str(yaml).unwrap();

    let _client_config: rustls::ClientConfig = (&config).try_into().unwrap();
}

#[test]
fn try_from_fails_for_invalid_config() {
    let yaml = r#"
allowed_versions:
  v1_2: false
  v1_3: false
"#;
    let config: TlsClientPolicyConfig = serde_yaml::from_str(yaml).unwrap();

    let result: Result<rustls::ClientConfig, _> = (&config).try_into();
    assert!(result.is_err());
}

#[test]
fn reqwest_compatibility() {
    let config = TlsClientPolicyConfig::default()
        .rustls_0_23_config()
        .unwrap();
    reqwest::Client::builder()
        .use_preconfigured_tls(config)
        .build()
        .expect("Failed to create reqwest client");
}
