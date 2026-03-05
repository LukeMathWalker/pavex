mod collection;
mod krate;

// Public API (consumed via `rustdoc/mod.rs` re-exports)
pub use collection::{CrateCollection, ResolvedItem};
pub use rustdoc_processor::GlobalItemId;
pub use rustdoc_processor::queries::{Crate, CrateRegistry};

// Crate-internal visibility
pub(in crate::rustdoc) use krate::index;
