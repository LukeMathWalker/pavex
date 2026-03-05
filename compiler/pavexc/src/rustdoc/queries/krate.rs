use std::collections::BTreeSet;

use guppy::PackageId;

use crate::diagnostic::DiagnosticSink;
use rustdoc_processor::{
    Crate, CrateData, CrateItemIndex, CrateItemPaths, EagerCrateItemIndex, EagerCrateItemPaths,
    IndexingVisitor,
};

use super::super::annotations::{
    self, AnnotatedItems, QueueItem, invalid_diagnostic_attribute, parse_pavex_attributes,
};

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

pub(in crate::rustdoc) fn index_raw(
    krate: rustdoc_types::Crate,
    package_id: PackageId,
    diagnostics: &DiagnosticSink,
) -> (Crate, AnnotatedItems) {
    let crate_data = CrateData {
        root_item_id: krate.root,
        index: CrateItemIndex::Eager(EagerCrateItemIndex { index: krate.index }),
        external_crates: krate.external_crates,
        format_version: krate.format_version,
        paths: CrateItemPaths::Eager(EagerCrateItemPaths { paths: krate.paths }),
    };
    index(crate_data, package_id, diagnostics)
}

pub(in crate::rustdoc) fn index(
    krate: CrateData,
    package_id: PackageId,
    diagnostics: &DiagnosticSink,
) -> (Crate, AnnotatedItems) {
    let mut annotation_queue = BTreeSet::<QueueItem>::new();
    let mut visitor = PavexIndexingVisitor {
        annotation_queue: &mut annotation_queue,
        diagnostics,
    };
    let krate = Crate::index(krate, package_id, &mut visitor);
    let annotated_items = annotations::process_queue(annotation_queue, &krate, diagnostics);
    (krate, annotated_items)
}
