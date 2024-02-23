extern crate core;

pub use compiler::App;
pub use persistence::AppWriter;

mod compiler;
mod diagnostic;
pub(crate) mod language;
mod persistence;
mod rustdoc;
mod utils;
