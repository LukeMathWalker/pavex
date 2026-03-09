mod collection;

// Public API (consumed via `rustdoc/mod.rs` re-exports)
pub use collection::{CrateCollection, ResolvedItem};
pub use rustdoc_processor::GlobalItemId;
pub use rustdoc_processor::queries::{Crate, CrateRegistry};
