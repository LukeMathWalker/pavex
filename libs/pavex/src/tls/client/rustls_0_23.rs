use crate::tls::client::config::{
    AllowedTlsVersionsConfig, CertificateVerificationConfig, CryptoProviderConfig, RootCertificate,
    RootCertificateFileEncoding, RootCertificateInlineEncoding,
};

use super::TlsClientPolicyConfig;
use anyhow::{Context, Result, bail, ensure};
use base64::{Engine, prelude::BASE64_STANDARD_NO_PAD};
use rustls::{
    ClientConfig, Error as TlsError, RootCertStore, SupportedProtocolVersion,
    client::{
        WebPkiServerVerifier,
        danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
    },
    crypto::CryptoProvider,
    pki_types::ServerName,
    version::{TLS12, TLS13},
};
use rustls_pki_types::{CertificateDer, pem::SectionKind};
use std::sync::Arc;

impl TryFrom<&TlsClientPolicyConfig> for ClientConfig {
    type Error = anyhow::Error;

    fn try_from(value: &TlsClientPolicyConfig) -> Result<Self, Self::Error> {
        value.rustls_0_23_config()
    }
}

impl TlsClientPolicyConfig {
    /// Build a [`rustls::ClientConfig`] according to the specified configuration.
    pub fn rustls_0_23_config(&self) -> Result<ClientConfig> {
        let provider = Arc::new(crypto_provider(&self.crypto_provider)?);
        let builder = ClientConfig::builder_with_provider(provider.clone())
            .with_protocol_versions(&supported_versions(self.allowed_versions)?)?;

        let config = if self.insecure.skip_verification {
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

        Ok(config)
    }
}

fn crypto_provider(p: &CryptoProviderConfig) -> Result<CryptoProvider> {
    match p {
        #[cfg(not(feature = "tls_crypto_provider_aws_lc_rs"))]
        CryptoProviderConfig::AwsLcRs { .. } => {
            bail!(
                "Your TLS client configuration wants to use `aws_lc_rs` as its cryptography stack, but the corresponding `cargo` feature is not enabled.\n\
                 Add `tls_crypto_provider_aws_lc_rs` to the `features` array for `pavex` in your Cargo.toml manifest."
            );
        }
        #[cfg(feature = "tls_crypto_provider_aws_lc_rs")]
        CryptoProviderConfig::AwsLcRs { require_fips } => {
            let prov = rustls::crypto::aws_lc_rs::default_provider();

            if *require_fips && !prov.fips() {
                bail!(
                    "FIPS requested but the `fips` feature is not enabled. Add `fips` to the `features` array for `pavex` in your Cargo.toml manifest."
                );
            }
            Ok(prov)
        }
        #[cfg(feature = "tls_crypto_provider_ring")]
        CryptoProviderConfig::Ring => Ok(rustls::crypto::ring::default_provider()),
        #[cfg(not(feature = "tls_crypto_provider_ring"))]
        CryptoProviderConfig::Ring => bail!(
            "Your TLS client configuration wants to use `ring` as its cryptography stack, but the corresponding `cargo` feature is not enabled.\n\
             Add `tls_crypto_provider_ring` to the `features` array for `pavex` in your Cargo.toml manifest."
        ),
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
    let additional_roots = additional_roots(&config.additional_roots)?;
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
    config: AllowedTlsVersionsConfig,
) -> Result<&'static [&'static SupportedProtocolVersion]> {
    static ALL: [&SupportedProtocolVersion; 2] = [&TLS12, &TLS13];
    static ONLY_TLS12: [&SupportedProtocolVersion; 1] = [&TLS12];
    static ONLY_TLS13: [&SupportedProtocolVersion; 1] = [&TLS13];

    match (config.v1_2, config.v1_3) {
        (true, true) => Ok(&ALL),
        (true, false) => Ok(&ONLY_TLS12),
        (false, true) => Ok(&ONLY_TLS13),
        (false, false) => {
            bail!("You disabled both TLS 1.2 and TLS 1.3. At least one of them must be enabled.");
        }
    }
}

fn additional_roots(root_sources: &[RootCertificate]) -> Result<Vec<CertificateDer<'static>>> {
    let mut roots = Vec::with_capacity(root_sources.len());
    for (i, source) in root_sources.iter().enumerate() {
        parse_additional_root(&mut roots, source).with_context(|| {
            format!(
                "Failed to parse the root certificate at index {i} in your list of `additional_roots`",
            )
        })?;
    }
    Ok(roots)
}

fn parse_additional_root(
    roots: &mut Vec<CertificateDer<'static>>,
    source: &RootCertificate,
) -> Result<()> {
    match source {
        RootCertificate::File { encoding, path } => {
            let contents = fs_err::read(path).context("Failed to read root certificate file")?;
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
    Ok(())
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
                "Expected a PEM-encoded root certificate, but found a {} section",
                kind2str(section_kind)
            )
        }
        certs.push(CertificateDer::from(section_data));
    }
    ensure!(
        !certs.is_empty(),
        "Your PEM bundle doesn't contain any root certificate. There should be at least one `BEGIN CERTIFICATE` block"
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
