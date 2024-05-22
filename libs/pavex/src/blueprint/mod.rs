//! Define the routes and the structure of your application using a [`Blueprint`].
//!
//! Check out the ["Project structure"](https://pavex.dev/docs/guide/project_structure) section of the
//! Pavex guide for more details on the role of [`Blueprint`] in Pavex applications.
pub use blueprint::Blueprint;

#[allow(clippy::module_inception)]
mod blueprint;
pub mod constructor;
mod conversions;
pub mod error_observer;
pub mod linter;
pub mod middleware;
pub mod reflection;
pub mod router;
pub mod state;
