//! Register configuration types that can be deserialized from your configuration sources
//! and injected into your components by Pavex.
//!
//! # Guide
//!
/// Check out the ["Configuration"](https://pavex.dev/docs/guide/configuration)
/// section of Pavex's guide for a thorough introduction to Pavex's configuration system.
mod registered;
mod unregistered;

pub use registered::RegisteredConfigType;
pub use unregistered::ConfigType;
