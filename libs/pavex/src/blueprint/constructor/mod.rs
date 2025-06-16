//! Register constructors for the types that can be injected into your request and error handlers.  
//!
//! # Guide
//!
//! Check out the ["Dependency injection"](https://pavex.dev/docs/guide/dependency_injection)
//! section of Pavex's guide for a thorough introduction to dependency injection
//! in Pavex applications.
pub use cloning_strategy::CloningStrategy;
pub use lifecycle::Lifecycle;
pub use registered::RegisteredConstructor;

mod cloning_strategy;
mod lifecycle;
mod registered;
