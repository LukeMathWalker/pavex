//! Callable metadata used by Pavex's CLI to analyze your request
//! handlers, constructors and error handlers (e.g. their input parameters, their return type,
//! where they are defined, etc.).
//!
//! This module is not meant to be used directly by users of the framework. It is only meant to be
//! used by Pavex's CLI.
pub use callable::{RawCallable, RawCallableIdentifiers};
pub use location::Location;

mod callable;
mod location;
