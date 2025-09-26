use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// Wrapped into a sub-module to avoid exposing `TlsClientPolicyConfig` in two places:
// inside `pavex::tls::config` and `pavex::tls::client`.
// We only want users to see `pavex::tls::client::TlsClientPolicyConfig`.
pub(crate) mod _config {
    use super::*;

    #[derive(Debug, Default, Clone, Deserialize, Serialize)]
    /// Configure the TLS policy for a client.
    ///
    /// It covers:
    /// - The [cryptographic stack](`Self::crypto_provider`) used to secure the connection.
    /// - Which [TLS versions](`Self::allowed_versions`) are allowed.
    /// - The [certificate verification](`Self::certificate_verification`) mechanism used to verify server certificates.
    ///
    /// For testing/development purposes only, it exposes a few [insecure](`Self::insecure`) configuration options
    /// that lower the security posture of your client.
    ///
    /// # Defaults
    ///
    /// The default configuration should be suitable for most production environments:
    ///
    #[doc = include_str!("../../../tests/fixtures/tls_config/default.yaml")]
    ///
    /// # Overriding the default configuration
    ///
    /// If you want to deviate from the default configuration, it's enough to specify the fields you
    /// want to override.
    ///
    /// ## Example: Disable TLS 1.2
    ///
    #[doc = include_str!("../../../tests/fixtures/tls_config/disable_tls_1_2.yaml")]
    ///
    /// ## Example: Trust additional root certificates
    ///
    #[doc = include_str!("../../../tests/fixtures/tls_config/additional_roots.yaml")]
    ///
    /// ## Example: Disable certificate verification
    ///
    #[doc = include_str!("../../../tests/fixtures/tls_config/skip_verification.yaml")]
    pub struct TlsClientPolicyConfig {
        /// The cryptographic stack used to secure the connection.
        ///
        /// Refer to the documentation for [`CryptoProviderConfig`](CryptoProviderConfig)
        /// for more details.
        #[serde(default)]
        #[serde(with = "serde_yaml::with::singleton_map_recursive")]
        pub crypto_provider: CryptoProviderConfig,
        /// Which TLS versions are allowed.
        ///
        /// Refer to the documentation for [`TlsVersionsConfig`](TlsVersionsConfig)
        /// for more details.
        #[serde(default)]
        pub allowed_versions: AllowedTlsVersionsConfig,
        /// The mechanism used to verify server certificates.
        ///
        /// Refer to the documentation for [`CertificateVerificationConfig`](CertificateVerificationConfig)
        /// for more details.
        #[serde(default)]
        pub certificate_verification: CertificateVerificationConfig,
        /// Dangerous configuration options that lower the security
        /// posture of your client.
        ///
        /// These options should never be used in production scenarios.
        /// They are available for testing/development purposes only.
        #[serde(default)]
        pub insecure: InsecureTlsClientConfig,
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[non_exhaustive]
/// Which TLS versions are allowed.
///
/// By default, TLS 1.2 and TLS 1.3 are enabled.
///
/// # Security
///
/// The lack of support for TLS 1.0 and TLS 1.1 is intentional.
pub struct AllowedTlsVersionsConfig {
    /// Enables TLS 1.2 if `true`.
    ///
    /// It requires the server to support TLS 1.2.
    #[serde(default = "default_v1_2")]
    pub v1_2: bool,
    /// Enables TLS 1.3 if `true`.
    ///
    /// It requires the server to support TLS 1.3.
    #[serde(default = "default_v1_3")]
    pub v1_3: bool,
}

fn default_v1_2() -> bool {
    true
}

fn default_v1_3() -> bool {
    true
}

impl Default for AllowedTlsVersionsConfig {
    fn default() -> Self {
        Self {
            v1_2: default_v1_2(),
            v1_3: default_v1_3(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
/// Configure how server certificates are verified.
///
/// # Default
///
/// By default, we rely on verification machinery of the underlying operating system.
/// Refer to the documentation for [`rustls-platform-verifier`](https://docs.rs/rustls-platform-verifier/latest/rustls_platform_verifier/)
/// for more information on how each platform handles certificate verification.
///
/// # Customization
///
/// Set [`additional_roots`][`CertificateVerificationConfig::additional_roots`] to trust
/// additional root certificates in addition to the ones already trusted
/// by the operating system.
///
/// # Skipping Verification
///
/// If you want to skip certificate verification altogether, check out the [`insecure`][`TlsClientConfig::insecure`]
/// options in [`TlsClientConfig`].
///
/// ## Incorrect Usage
///
/// Setting [`use_os_verifier`][`CertificateVerificationConfig::use_os_verifier`] to `false`, with
/// no [`additional_roots`][`CertificateVerificationConfig::additional_roots`] specified, does **not**
/// disable certificate verification. It does instead cause all certificate verification attempts to fail.
///
/// We treat this scenario as a misconfiguration and return an error at runtime, when
/// trying to initialize the client.
pub struct CertificateVerificationConfig {
    /// Whether to use the certificate verification machinery provided by
    /// the underlying operating system.
    ///
    /// Defaults to `true`.
    #[serde(default = "default_use_os_verifier")]
    pub use_os_verifier: bool,
    /// Trust one or more additional root certificates.
    ///
    /// If [`use_os_verifier`][`CertificateVerificationConfig::use_os_verifier`] is `false`,
    /// these will be the only trusted root certificates.
    /// If [`use_os_verifier`][`CertificateVerificationConfig::use_os_verifier`] is `true`, these will be
    /// trusted **in addition** to the ones already trusted by the underlying operating system.
    ///
    /// They can either be loaded from files or inlined in configuration.
    #[serde(default)]
    #[serde(with = "serde_yaml::with::singleton_map_recursive")]
    pub additional_roots: Vec<RootCertificate>,
}

fn default_use_os_verifier() -> bool {
    true
}

impl Default for CertificateVerificationConfig {
    fn default() -> Self {
        CertificateVerificationConfig {
            use_os_verifier: default_use_os_verifier(),
            additional_roots: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum RootCertificate {
    File {
        encoding: RootCertificateFileEncoding,
        path: PathBuf,
    },
    Inline {
        encoding: RootCertificateInlineEncoding,
        data: String,
    },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
/// Supported encodings for the root certificate in [`RootCertificate::File`].
pub enum RootCertificateFileEncoding {
    /// A DER-encoded X.509 certificate; as specified in [RFC 5280](https://datatracker.ietf.org/doc/html/rfc5280#section-4.1).
    Der,
    /// A PEM-encoded X.509 certificate; as specified in [RFC 7468](https://datatracker.ietf.org/doc/html/rfc7468#section-5).
    Pem,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
/// Supported encodings for the root certificate in [`RootCertificate::Inline`].
pub enum RootCertificateInlineEncoding {
    /// A DER-encoded X.509 certificate; as specified in [RFC 5280](https://datatracker.ietf.org/doc/html/rfc5280#section-4.1).
    ///
    /// Since DER is a binary format, we expect the data to be [base64-encoded](https://datatracker.ietf.org/doc/html/rfc4648#section-4).
    Base64Der,
    /// A PEM-encoded X.509 certificate; as specified in [RFC 7468](https://datatracker.ietf.org/doc/html/rfc7468#section-5).
    ///
    /// Since PEM is a text format, we don't expect the data to be base64-encoded.
    Pem,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[non_exhaustive]
pub struct InsecureTlsClientConfig {
    /// Don't verify server certificates.
    ///
    /// Extremely dangerous option, limit its usage to local development environments.
    #[serde(default = "default_skip_verification")]
    pub skip_verification: bool,
}

impl Default for InsecureTlsClientConfig {
    fn default() -> Self {
        InsecureTlsClientConfig {
            skip_verification: default_skip_verification(),
        }
    }
}

fn default_skip_verification() -> bool {
    false
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", tag = "name")]
#[non_exhaustive]
pub enum CryptoProviderConfig {
    AwsLcRs {
        #[serde(default)]
        require_fips: bool,
    },
    Ring,
}

impl Default for CryptoProviderConfig {
    fn default() -> Self {
        CryptoProviderConfig::AwsLcRs {
            require_fips: false,
        }
    }
}
