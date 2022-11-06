// Re-export the dependencies that we use in the generated application code.
pub use anyhow::Error;
pub use http;
pub use hyper;
pub use matchit as routing;

// A dirty hack to make sure that `pavex_runtime` ends up in the generated
// Cargo.toml
pub struct Placeholder;
