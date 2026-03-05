use std::collections::BTreeSet;

use guppy::PackageId;
use indexmap::IndexSet;

use crate::diagnostic::DiagnosticSink;
use rustdoc_processor::{CrateData, ExternalReExports, ImportIndex, IndexingVisitor};

use super::super::annotations::{QueueItem, invalid_diagnostic_attribute, parse_pavex_attributes};

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

pub(super) fn index_local_types<'a>(
    krate: &'a CrateData,
    package_id: &'a PackageId,
    navigation_history: IndexSet<rustdoc_types::Id>,
    current_path: Vec<String>,
    import_index: &mut ImportIndex,
    re_exports: &mut ExternalReExports,
    annotation_queue: &mut BTreeSet<QueueItem>,
    current_item_id: &rustdoc_types::Id,
    is_public: bool,
    renamed_to: Option<String>,
    encountered_use: bool,
    diagnostics: &DiagnosticSink,
) {
    let mut visitor = PavexIndexingVisitor {
        annotation_queue,
        diagnostics,
    };
    rustdoc_processor::index_local_types(
        krate,
        package_id,
        navigation_history,
        current_path,
        import_index,
        re_exports,
        &mut visitor,
        current_item_id,
        is_public,
        renamed_to,
        encountered_use,
    );
}
