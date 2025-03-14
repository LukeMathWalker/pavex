//! Scan local and foreign modules to find macro-annotated constructors, configuration types
//! and error handlers.
//!
//! # Guide
//!
//! Check out the ["Dependency injection"](https://pavex.dev/docs/guide/dependency_injection)
//! section of Pavex's guide for a thorough introduction to dependency injection
//! in Pavex applications.
pub use registered::RegisteredImport;
pub use unregistered::Import;

mod registered;
mod unregistered;
