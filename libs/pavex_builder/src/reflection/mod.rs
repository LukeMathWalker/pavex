//! Internal machinery to keep track of the metadata required by `pavex` to later analyze your request
//! handlers, constructors and error handlers (e.g. their input parameters, their return type,
//! where they are defined, etc.).
//!
//! This module is not meant to be used directly by users of the framework. It is only meant to be
//! used by the `pavex` code generator.
pub use callable::{RawCallable, RawCallableIdentifiers};
pub use location::Location;

mod callable;
mod location;
