//! Utilities to log common error properties with consistent naming and representations.
//!
//! For well-known properties, this module exposes:
//!
//! - A constant holding the conventional field name used when logging that resource
//! - A function to compute the conventional log representation of that resource
//!
//! For example, you have [`ERROR_MESSAGE`] and [`error_message`] for the
//! `error.message` field.
use tracing::Value;

/// The field name to record the `Display` representation of an error.
///
/// Use [`error_message`] to populate the field.
pub const ERROR_MESSAGE: &str = "error.message";

/// The field name to record the `Debug` representation of an error.
///
/// Use [`error_message`] to populate the field.
pub const ERROR_DETAILS: &str = "error.details";

/// The field name to record the chain of sources for an error.
///
/// Use [`error_source_chain`] to populate the field.
pub const ERROR_SOURCE_CHAIN: &str = "error.source_chain";

/// The canonical representation for the value in [`ERROR_MESSAGE`].
pub fn error_message<E: std::fmt::Display>(e: E) -> impl Value {
    tracing::field::display(e)
}

/// The canonical representation for the value in [`ERROR_DETAILS`].
pub fn error_details<E: std::fmt::Debug>(e: E) -> impl Value {
    tracing::field::debug(e)
}

/// The canonical representation for the value in [`ERROR_SOURCE_CHAIN`].
pub fn error_source_chain<E: std::error::Error>(e: E) -> impl Value {
    _error_source_chain(e)
}

fn _error_source_chain<E: std::error::Error>(e: E) -> String {
    use std::fmt::Write as _;

    let mut chain = String::new();
    let mut source = e.source();
    while let Some(s) = source {
        let _ = writeln!(chain, "- {}", s);
        source = s.source();
    }
    chain
}
