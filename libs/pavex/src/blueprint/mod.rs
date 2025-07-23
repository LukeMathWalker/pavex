//! Define the structure of your application using a [`Blueprint`].
//!
//! Check out the ["Project structure"](https://pavex.dev/docs/guide/project_structure) section of the
//! Pavex guide for more details on the role of [`Blueprint`] in Pavex applications.
//!
//! [`Blueprint`]: crate::Blueprint

/// Capture a list of sources to be checked by Pavex for components.
///
/// `from!` is used by [`Blueprint::import`] and [`Blueprint::routes`] to determine
/// which modules should be examined.
///
/// [`Blueprint::import`]: crate::Blueprint::import
/// [`Blueprint::routes`]: crate::Blueprint::routes
pub use pavex_macros::from;

#[allow(clippy::module_inception)]
pub(super) mod blueprint;
pub use cloning_policy::CloningPolicy;
pub use lifecycle::Lifecycle;

pub use config::*;
pub use constructor::*;
pub use error_handler::*;
pub use error_observer::*;
pub use fallback::*;
pub use import::*;
pub use lints::Lint;
pub use nesting::RoutingModifiers;
pub use post::*;
pub use pre::*;
pub use prebuilt::*;
pub use route::*;
pub use routes::*;
pub use wrapping::*;

mod cloning_policy;
mod config;
mod constructor;
mod conversions;
mod error_handler;
mod error_observer;
mod fallback;
mod import;
mod lifecycle;
mod lints;
mod nesting;
mod post;
mod pre;
mod prebuilt;
pub mod reflection;
mod route;
mod routes;
mod wrapping;
