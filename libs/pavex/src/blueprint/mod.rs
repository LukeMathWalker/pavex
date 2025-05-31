//! Define the routes and the structure of your application using a [`Blueprint`].
//!
//! Check out the ["Project structure"](https://pavex.dev/docs/guide/project_structure) section of the
//! Pavex guide for more details on the role of [`Blueprint`] in Pavex applications.
pub use blueprint::Blueprint;
/// Capture a list of sources to be checked by Pavex for components.
///
/// It is used by [`Blueprint::import`] to determine which modules should
/// be scanned for macro-annotated constructors, error handlers and configuration types.
pub use pavex_macros::from;

#[allow(clippy::module_inception)]
mod blueprint;
pub mod config;
pub mod constructor;
mod conversions;
pub mod error_handler;
pub mod error_observer;
pub mod import;
pub mod linter;
pub mod middleware;
pub mod nesting;
pub mod prebuilt;
pub mod reflection;
pub mod router;
