mod collection;

// Public API (consumed via `rustdoc/mod.rs` re-exports)
pub type CrateCollection = rustdoc_processor::CrateCollection<super::indexer::PavexIndexer>;

pub use collection::{CrateCollectionExt, ResolvedItem};
pub use rustdoc_processor::GlobalItemId;
pub use rustdoc_processor::queries::{Crate, CrateRegistry};
