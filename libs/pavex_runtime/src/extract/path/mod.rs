//! Extract the values of templated path segments from the incoming request using [`Path`].
pub use extractor::{ExtractPathError, InvalidUtf8InPathParameter, Path};

mod deserializer;
mod extractor;
