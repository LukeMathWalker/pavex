//! Core rustdoc data types.

use rustc_hash::FxHashMap;
use rustdoc_types::ExternalCrate;

use super::{CrateItemIndex, CrateItemPaths};

/// The JSON documentation for a crate.
#[derive(Debug, Clone)]
pub struct CrateData {
    /// The id of the root item for the crate.
    pub root_item_id: rustdoc_types::Id,
    /// A mapping from the id of an external crate to the information about it.
    #[allow(clippy::disallowed_types)]
    pub external_crates: FxHashMap<u32, ExternalCrate>,
    /// A mapping from the id of a type to its fully qualified path.
    /// Primarily useful for foreign items that are being re-exported by this crate.
    pub paths: CrateItemPaths,
    /// The version of the JSON format used by rustdoc.
    pub format_version: u32,
    /// The index of all the items in the crate.
    pub index: CrateItemIndex,
}
