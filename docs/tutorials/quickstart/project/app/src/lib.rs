// If a module defines a component (e.g. a route or a middleware or a constructor), it must be
// public. Those components must be importable from the `server_sdk` crate, therefore they must
// be accessible from outside this crate.
mod blueprint;
pub mod configuration;
pub mod routes;
pub mod telemetry;

pub use blueprint::blueprint;
