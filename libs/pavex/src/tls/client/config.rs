use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
pub struct TlsClientConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    #[serde(default)]
    pub crypto_provider: CryptoProviderConfig,

    #[serde(default)]
    pub alpn: Vec<String>,

    #[serde(default = "default_min_tls_version")]
    pub min_tls_version: TlsProtocolVersion,

    #[serde(default = "default_max_tls_version")]
    pub max_tls_version: TlsProtocolVersion,

    /// SNI/verification host override
    #[serde(default)]
    pub tls_server_name: Option<String>,

    /// The mechanism used to verify server certificates.
    #[serde(default)]
    pub certificate_verification: CertificateVerificationConfig,

    #[serde(default)]
    pub insecure: InsecureTlsClientConfig,
}

fn default_enabled() -> bool {
    true
}

fn default_max_tls_version() -> TlsProtocolVersion {
    TlsProtocolVersion::V1_3
}

fn default_min_tls_version() -> TlsProtocolVersion {
    TlsProtocolVersion::V1_3
}

#[derive(Debug, Clone, Deserialize)]
/// Configure how server certificates are verified.
///
/// # Default
///
/// By default, the operating system's certificate verification machinery is used.
///
/// Refer to the documentation for [`rustls-platform-verifier`](https://docs.rs/rustls-platform-verifier/latest/rustls_platform_verifier/)
/// for more information on how each platform handles certificate verification.
///
/// # Customization
///
/// Set [`additional_roots`][`CertificateVerificationConfig::additional_roots`] to trust
/// additional root certificates in addition to the ones already trusted
/// by the underlying operating system.
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
    pub additional_roots: Vec<RootCertificate>,
}

fn default_use_os_verifier() -> bool {
    true
}

impl Default for CertificateVerificationConfig {
    fn default() -> Self {
        CertificateVerificationConfig {
            use_os_verifier: default_use_os_verifier(),
            additional_roots: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "source", rename_all = "snake_case")]
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

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum RootCertificateFileEncoding {
    /// A DER-encoded X.509 certificate; as specified in [RFC 5280](https://datatracker.ietf.org/doc/html/rfc5280#section-4.1).
    Der,
    /// A PEM-encoded X.509 certificate; as specified in [RFC 7468](https://datatracker.ietf.org/doc/html/rfc7468#section-5).
    Pem,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
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

#[derive(Debug, Clone, Deserialize)]
pub struct InsecureTlsClientConfig {
    /// Don't verify the server's certificate.
    ///
    /// Extremely dangerous option, limit its usage to local development environments.
    #[serde(default = "default_skip_verify")]
    pub skip_verification: bool,
}

impl Default for InsecureTlsClientConfig {
    fn default() -> Self {
        InsecureTlsClientConfig {
            skip_verification: default_skip_verify(),
        }
    }
}

fn default_skip_verify() -> bool {
    false
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind")]
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

/// The TLS protocol versions we support on the client-side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TlsProtocolVersion {
    /// TLS 1.2.
    ///
    /// In configuration, it is represented as "1.2".
    V1_2 = 0,
    /// TLS 1.3.
    ///
    /// In configuration, it is represented as "1.3".
    V1_3 = 1,
}

impl std::fmt::Display for TlsProtocolVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TlsProtocolVersion::V1_2 => write!(f, "1.2"),
            TlsProtocolVersion::V1_3 => write!(f, "1.3"),
        }
    }
}

impl<'de> serde::Deserialize<'de> for TlsProtocolVersion {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "1.2" => Ok(TlsProtocolVersion::V1_2),
            "1.3" => Ok(TlsProtocolVersion::V1_3),
            _ => Err(serde::de::Error::custom(format!(
                "Unknown protocol version: {}. \"1.2\" and \"1.3\" are the only supported TLS versions.",
                s
            ))),
        }
    }
}
