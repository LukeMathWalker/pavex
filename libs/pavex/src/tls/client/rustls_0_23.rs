use crate::tls::client::config::{
    CertificateVerificationConfig, CryptoProviderConfig, RootCertificate,
    RootCertificateFileEncoding, RootCertificateInlineEncoding,
};

use super::{TlsClientConfig, config::TlsProtocolVersion};
use anyhow::{Context, Result, bail, ensure};
use base64::{Engine, prelude::BASE64_STANDARD_NO_PAD};
use rustls::{
    ClientConfig, Error as TlsError, RootCertStore, SupportedProtocolVersion,
    client::{
        WebPkiServerVerifier,
        danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
    },
    crypto::{CryptoProvider, aws_lc_rs, ring},
    pki_types::ServerName,
    version::{TLS12, TLS13},
};
use rustls_pki_types::{CertificateDer, pem::SectionKind};
use std::sync::Arc;

impl TlsProtocolVersion {
    fn as_supported_protocol_version(&self) -> &'static SupportedProtocolVersion {
        match self {
            TlsProtocolVersion::V1_2 => &TLS12,
            TlsProtocolVersion::V1_3 => &TLS13,
        }
    }
}

/// =====================
/// Public API
/// =====================

impl TryFrom<&TlsClientConfig> for ClientConfig {
    type Error = anyhow::Error;

    fn try_from(value: &TlsClientConfig) -> Result<Self, Self::Error> {
        value.rustls_0_23_config()
    }
}

impl TlsClientConfig {
    /// Build a rustls ClientConfig from this policy (no globals required).
    pub fn rustls_0_23_config(&self) -> Result<ClientConfig> {
        ensure!(self.enabled, "TLS disabled by config");

        let provider = Arc::new(crypto_provider(&self.crypto_provider)?);
        let builder =
            ClientConfig::builder_with_provider(provider.clone()).with_protocol_versions(
                &supported_versions(self.min_tls_version, self.max_tls_version)?,
            )?;

        let mut config = if self.insecure.skip_verification {
            builder
                .dangerous()
                .with_custom_certificate_verifier(Arc::new(NoVerify { provider }))
        } else {
            let certificate_verifier =
                certificate_verifier(provider, &self.certificate_verification)?;
            builder
                .dangerous()
                .with_custom_certificate_verifier(certificate_verifier)
        }
        .with_no_client_auth();

        // ALPN (optional)
        if !self.alpn.is_empty() {
            config.alpn_protocols = self.alpn.iter().map(|s| s.as_bytes().to_vec()).collect();
        }

        Ok(config)
    }

    /// Resolve the ServerName used for SNI / name verification.
    pub fn resolve_server_name(&self, target_host: &str) -> Result<ServerName<'static>> {
        let name = self
            .tls_server_name
            .as_deref()
            .unwrap_or(target_host)
            .to_owned();
        Ok(name.try_into()?)
    }
}

fn crypto_provider(p: &CryptoProviderConfig) -> Result<CryptoProvider> {
    match p {
        CryptoProviderConfig::AwsLcRs { require_fips } => {
            let prov = aws_lc_rs::default_provider();
            if *require_fips && !prov.fips() {
                bail!("FIPS requested but rustls wasn't built with `fips` feature");
            }
            Ok(prov)
        }
        CryptoProviderConfig::Ring => Ok(ring::default_provider()),
    }
}

fn certificate_verifier(
    crypto_provider: Arc<CryptoProvider>,
    config: &CertificateVerificationConfig,
) -> Result<Arc<dyn ServerCertVerifier>> {
    if !config.use_os_verifier && config.additional_roots.is_empty() {
        anyhow::bail!(
            "You disabled OS server certificate verification without providing a list of additional root certificates to trust.\n\
           This configuration is invalid: it would cause **all** TLS connections to fail.\n\
           Please enable OS certificate verification or provide a list of root certificates to trust. Check out the documentation \
           of `pavex::tls::client::CertificateVerificationConfig` for more information."
        )
    }
    let additional_roots = additional_roots(&config.additional_roots)
        .context("Failed to parse the additional root certificates you provided")?;
    if config.use_os_verifier {
        let verifier = rustls_platform_verifier::Verifier::new_with_extra_roots(
            additional_roots,
            crypto_provider,
        )
        .context("Failed to initialize the server certificate verifier")?;
        Ok(Arc::new(verifier))
    } else {
        let mut root_cert_store = RootCertStore::empty();
        for root in additional_roots {
            root_cert_store
                .add(root)
                .context("One of your additional root certificates is invalid")?;
        }
        let verifier =
            WebPkiServerVerifier::builder_with_provider(Arc::new(root_cert_store), crypto_provider)
                .build()
                .context("Failed to initialize the server certificate verifier")?;
        Ok(verifier)
    }
}

fn supported_versions(
    min: TlsProtocolVersion,
    max: TlsProtocolVersion,
) -> Result<Vec<&'static SupportedProtocolVersion>> {
    if min > max {
        bail!(
            "The range of TLS protocol versions you specified is invalid.\n\
            Your minimum TLS version ({min}) is strictly greater than your maximum TLS version ({max}).",
        );
    }
    let range = [TlsProtocolVersion::V1_2, TlsProtocolVersion::V1_3]
        .into_iter()
        .filter_map(|v| {
            if v >= min && v <= max {
                Some(v.as_supported_protocol_version())
            } else {
                None
            }
        })
        .collect();
    Ok(range)
}

fn additional_roots(root_sources: &[RootCertificate]) -> Result<Vec<CertificateDer<'static>>> {
    let mut roots = Vec::with_capacity(root_sources.len());
    for source in root_sources {
        match source {
            RootCertificate::File { encoding, path } => {
                let contents =
                    fs_err::read(path).context("Failed to read root certificate file")?;
                match encoding {
                    RootCertificateFileEncoding::Der => {
                        roots.push(CertificateDer::from(contents));
                    }
                    RootCertificateFileEncoding::Pem => {
                        roots.extend(parse_certificates_from_pem_bytes(&contents)?);
                    }
                }
            }
            RootCertificate::Inline { encoding, data } => match encoding {
                RootCertificateInlineEncoding::Base64Der => {
                    let decoded = BASE64_STANDARD_NO_PAD
                        .decode(data)
                        .context("Failed to decode base64-encoded DER certificate")?;
                    roots.push(CertificateDer::from(decoded));
                }
                RootCertificateInlineEncoding::Pem => {
                    roots.extend(parse_certificates_from_pem_bytes(data.as_bytes())?);
                }
            },
        }
    }
    Ok(roots)
}

fn parse_certificates_from_pem_bytes(data: &[u8]) -> Result<Vec<CertificateDer<'static>>> {
    let mut certs = Vec::new();
    for outcome in
        <(SectionKind, Vec<u8>) as rustls_pki_types::pem::PemObject>::pem_slice_iter(data)
    {
        let (section_kind, section_data) =
            outcome.context("Failed to parse a section of your PEM-encoded root certificate")?;
        if section_kind != SectionKind::Certificate {
            anyhow::bail!(
                "Expected a PEM-encoded certificate, but found a {} section",
                kind2str(section_kind)
            )
        }
        certs.push(CertificateDer::from(section_data));
    }
    ensure!(
        !certs.is_empty(),
        "Your PEM bundle doesn't contain any CERTIFICATE blocks"
    );
    Ok(certs)
}

fn kind2str(kind: SectionKind) -> &'static str {
    match kind {
        SectionKind::Certificate => "CERTIFICATE",
        SectionKind::PublicKey => "PUBLIC KEY",
        SectionKind::RsaPrivateKey => "RSA PRIVATE KEY",
        SectionKind::PrivateKey => "PRIVATE KEY",
        SectionKind::EcPrivateKey => "EC PRIVATE KEY",
        SectionKind::Crl => "X509 CRL",
        SectionKind::Csr => "CERTIFICATE REQUEST",
        SectionKind::EchConfigList => "ECHCONFIG",
        _ => "unknown",
    }
}

#[derive(Debug, Clone)]
/// A custom verifier that doesn't actually verify server certificates.
struct NoVerify {
    provider: Arc<CryptoProvider>,
}

impl ServerCertVerifier for NoVerify {
    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> std::result::Result<rustls::client::danger::HandshakeSignatureValid, TlsError> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> std::result::Result<rustls::client::danger::HandshakeSignatureValid, TlsError> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        self.provider
            .signature_verification_algorithms
            .supported_schemes()
    }

    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls_pki_types::UnixTime,
    ) -> std::result::Result<ServerCertVerified, TlsError> {
        Ok(ServerCertVerified::assertion())
    }
}
