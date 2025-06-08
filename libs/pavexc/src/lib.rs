#![allow(clippy::too_many_arguments)]
extern crate core;

pub use compiler::App;
pub use diagnostic::DiagnosticSink;
pub use persistence::AppWriter;

mod compiler;
mod diagnostic;
pub(crate) mod language;
mod persistence;
pub mod rustdoc;
mod utils;

/// The Rust toolchain used by `pavexc` to generate JSON docs, unless
/// overridden by the user.
pub static DEFAULT_DOCS_TOOLCHAIN: &str = "nightly-2025-03-26";
