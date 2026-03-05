mod collection;
mod indexing;
mod krate;

// Public API (consumed via `rustdoc/mod.rs` re-exports)
pub use collection::{CrateCollection, ResolvedItem};
pub use rustdoc_processor::{Crate, CrateRegistry, GlobalItemId};

// Crate-internal visibility
pub(in crate::rustdoc) use krate::index;
