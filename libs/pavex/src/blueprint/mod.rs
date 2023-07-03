//! Define the routes and the structure of your application using a [`Blueprint`].
pub use blueprint::Blueprint;

mod blueprint;
pub mod constructor;
pub mod internals;
pub mod reflection;
pub mod router;