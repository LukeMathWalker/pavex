//! Tracking of external re-exports.

use ahash::HashMap;

use crate::CrateCollection;
use crate::compute::CannotGetCrateData;
use crate::crate_data::CrateData;
use crate::indexing::CrateIndexer;
use crate::queries::Crate;
use rustdoc_ext::GlobalItemId;

/// Track re-exports of types (or entire modules!) from other crates.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ExternalReExports {
    /// Key: the path of the re-exported type in the current crate.
    /// Value: the id of the `rustdoc` item of kind `use` that performed the re-export.
    ///
    /// E.g. `pub use hyper::server as sx;` in `lib.rs` would use `vec!["my_crate", "sx"]`
    /// as key in this map.
    pub(crate) target_path2use_id: HashMap<Vec<String>, rustdoc_types::Id>,
    /// Key: the id of the `rustdoc` item of kind `use` that performed the re-export.
    /// Value: metadata about the re-export.
    pub(crate) use_id2re_export: HashMap<rustdoc_types::Id, ExternalReExport>,
}

impl ExternalReExports {
    /// Iterate over the external re-exports that have been collected.
    pub fn iter(
        &self,
    ) -> impl Iterator<Item = (&Vec<String>, rustdoc_types::Id, &ExternalReExport)> {
        self.target_path2use_id
            .iter()
            .map(|(target_path, id)| (target_path, *id, &self.use_id2re_export[id]))
    }

    /// Get metadata about a re-export given the use item id.
    pub fn get(&self, use_id: &rustdoc_types::Id) -> Option<&ExternalReExport> {
        self.use_id2re_export.get(use_id)
    }

    /// Get the use item id for a given target path.
    pub fn get_use_id(&self, target_path: &[String]) -> Option<rustdoc_types::Id> {
        self.target_path2use_id.get(target_path).copied()
    }

    /// Insert a re-export entry.
    pub fn insert_entry(
        &mut self,
        target_path: Vec<String>,
        use_id: rustdoc_types::Id,
        re_export: ExternalReExport,
    ) {
        self.target_path2use_id.insert(target_path, use_id);
        self.use_id2re_export.insert(use_id, re_export);
    }

    /// Retrieve the re-exported item from the crate it was defined into.
    ///
    /// # Panics
    ///
    /// Panics if the provided `use_id` doesn't exist as a key in the re-export registry.
    pub fn get_target_item_id<I: CrateIndexer>(
        &self,
        // The crate associated with these re-exports.
        re_exported_from: &Crate,
        collection: &CrateCollection<I>,
        use_id: rustdoc_types::Id,
    ) -> Result<Option<GlobalItemId>, CannotGetCrateData> {
        let re_export = self
            .get(&use_id)
            .expect("use_id not found in re-export registry");
        let source_package_id = re_exported_from
            .core
            .compute_package_id_for_crate_id(
                re_export.external_crate_id,
                collection.package_graph(),
                None,
            )
            .expect("Failed to compute the package id for a given external crate id");
        let source_krate = collection.get_or_compute(&source_package_id)?;
        let Ok(Ok(source_id)) =
            source_krate.get_item_id_by_path(&re_export.source_path, collection)
        else {
            return Ok(None);
        };
        Ok(Some(source_id))
    }

    /// Add another re-export to the database.
    pub fn insert(
        &mut self,
        krate: &CrateData,
        use_item: &rustdoc_types::Item,
        current_path: &[String],
    ) {
        let rustdoc_types::ItemEnum::Use(use_) = &use_item.inner else {
            unreachable!()
        };
        let imported_id = use_.id.expect("Import doesn't have an associated id");
        let Some(imported_summary) = krate.paths.get(&imported_id) else {
            // TODO: this is firing for std's JSON docs. File a bug report.
            // panic!("The imported id ({}) is not listed in the index nor in the path section of rustdoc's JSON output", imported_id.0)
            return;
        };
        debug_assert!(imported_summary.crate_id != 0);
        // We are looking at a public re-export of another crate
        // (e.g. `pub use hyper;`), one of its modules or one of its items.
        // Due to how re-exports are handled in `rustdoc`, the re-exported
        // items inside that foreign module will not be found in the `index`
        // for this crate.
        // We intentionally add foreign items to the index to get a "complete"
        // picture of all the types available in this crate.
        let external_crate_id = imported_summary.crate_id;
        let source_path = imported_summary.path.to_owned();
        let re_exported_path = {
            let mut p = current_path.to_owned();
            if !use_.is_glob {
                p.push(use_.name.clone());
            }
            p
        };
        let re_export = ExternalReExport {
            source_path,
            external_crate_id,
        };

        self.target_path2use_id
            .insert(re_exported_path, use_item.id);
        self.use_id2re_export.insert(use_item.id, re_export);
    }
}

/// Information about a type (or module) re-exported from another crate.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, bincode::Encode, bincode::Decode)]
pub struct ExternalReExport {
    /// The path of the re-exported type in the crate it was re-exported from.
    ///
    /// E.g. `pub use hyper::server as sx;` in `lib.rs` would set `source_path` to
    /// `vec!["hyper", "server"]`.
    pub source_path: Vec<String>,
    /// The id of the source crate in the `external_crates` section of the JSON
    /// documentation of the crate that re-exported it.
    pub external_crate_id: u32,
}
