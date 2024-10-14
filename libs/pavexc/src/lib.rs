extern crate core;

pub use compiler::App;
pub use persistence::AppWriter;

mod compiler;
mod diagnostic;
pub(crate) mod language;
mod persistence;
pub mod rustdoc;
mod utils;

/// The Rust toolchain used by `pavexc` to generate JSON docs, unless
/// overridden by the user.
pub static DEFAULT_DOCS_TOOLCHAIN: &str = "nightly-2024-10-13";
