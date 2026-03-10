use std::collections::BTreeSet;

use guppy::PackageId;
use rustdoc_processor::crate_data::{
    CrateData, CrateItemIndex, CrateItemPaths, EagerCrateItemIndex, EagerCrateItemPaths,
};
use rustdoc_processor::indexing::{CrateIndexer, IndexResult, IndexingVisitor};
use rustdoc_processor::queries::Crate;

use crate::diagnostic::DiagnosticSink;

use super::annotations::{
    self, AnnotatedItems, QueueItem, invalid_diagnostic_attribute, parse_pavex_attributes,
};

/// Pavex-specific crate indexer that extracts `#[pavex(...)]` annotations.
pub struct PavexIndexer {
    pub(super) diagnostic_sink: DiagnosticSink,
}

impl PavexIndexer {
    pub fn new(diagnostic_sink: DiagnosticSink) -> Self {
        Self { diagnostic_sink }
    }
}

impl CrateIndexer for PavexIndexer {
    type Annotations = AnnotatedItems;

    fn index_raw(
        &self,
        krate: rustdoc_types::Crate,
        package_id: PackageId,
    ) -> IndexResult<AnnotatedItems> {
        let crate_data = CrateData {
            root_item_id: krate.root,
            index: CrateItemIndex::Eager(EagerCrateItemIndex { index: krate.index }),
            external_crates: krate.external_crates,
            format_version: krate.format_version,
            paths: CrateItemPaths::Eager(EagerCrateItemPaths { paths: krate.paths }),
        };
        self.index(crate_data, package_id)
    }

    fn index(&self, crate_data: CrateData, package_id: PackageId) -> IndexResult<AnnotatedItems> {
        let n_diagnostics = self.diagnostic_sink.len();
        let mut annotation_queue = BTreeSet::<QueueItem>::new();
        let mut visitor = PavexIndexingVisitor {
            annotation_queue: &mut annotation_queue,
            diagnostics: &self.diagnostic_sink,
        };
        let krate = Crate::index(crate_data, package_id, &mut visitor);
        let annotated_items =
            annotations::process_queue(annotation_queue, &krate, &self.diagnostic_sink);
        // No issues arose in the indexing phase if the diagnostic count hasn't changed.
        //
        // TODO: Since we're indexing in parallel, the counter may have been incremented
        //  by a different thread, signaling an issue with indexes for another crate.
        //  It'd be enough to keep a thread-local counter to get an accurate yes/no,
        //  but since we don't get false negatives it isn't a big deal.
        let can_cache_indexes = n_diagnostics == self.diagnostic_sink.len();
        IndexResult {
            krate,
            annotations: annotated_items,
            can_cache_indexes,
        }
    }
}

struct PavexIndexingVisitor<'a> {
    annotation_queue: &'a mut BTreeSet<QueueItem>,
    diagnostics: &'a DiagnosticSink,
}

impl IndexingVisitor for PavexIndexingVisitor<'_> {
    fn on_item_discovered(&mut self, item: &rustdoc_types::Item, item_id: rustdoc_types::Id) {
        match parse_pavex_attributes(&item.attrs) {
            Ok(Some(_)) => {
                self.annotation_queue.insert(QueueItem::Standalone(item_id));
            }
            Ok(None) => {}
            Err(e) => {
                // TODO: Only report an error if it's a crate from the current workspace
                invalid_diagnostic_attribute(e, item, self.diagnostics);
            }
        }
    }

    fn on_type_indexed(&mut self, item_id: rustdoc_types::Id) {
        self.annotation_queue.insert(QueueItem::Standalone(item_id));
    }
}
