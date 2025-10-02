//! Secure your client connections with TLS (Transport Layer Security).
//!
//! Check out the documentation for [`TlsClientPolicyConfig`] for
//! a detailed explanation of the available configuration options.
pub mod config;
pub use config::_config::TlsClientPolicyConfig;

#[cfg(feature = "rustls_0_23")]
mod rustls_0_23;
