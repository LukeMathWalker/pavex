mod collection;
mod indexing;
mod krate;
mod resolution;

// Public API (consumed via `rustdoc/mod.rs` re-exports)
pub use collection::{CrateCollection, ResolvedItem};
pub use krate::{Crate, ExternalReExportsExt};
pub use rustdoc_cache::GlobalItemId;

// Crate-internal visibility
pub(crate) use krate::CrateCore;
