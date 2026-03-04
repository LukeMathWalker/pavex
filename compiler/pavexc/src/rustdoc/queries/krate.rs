use std::collections::BTreeSet;

use ahash::HashMap;
use guppy::PackageId;
use rustdoc_types::ItemKind;

use crate::diagnostic::DiagnosticSink;
use rustdoc_cache::{
    Crate, CrateCore, CrateData, CrateItemIndex, CrateItemPaths, EagerCrateItemIndex,
    EagerCrateItemPaths, EagerImportPath2Id, ImportIndex, ImportPath2Id,
};

use super::super::annotations::{self, AnnotatedItems, QueueItem};
use super::indexing::index_local_types;
use indexmap::IndexSet;

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

#[tracing::instrument(skip_all, name = "index_crate_docs", fields(package.id = package_id.repr()))]
pub(in crate::rustdoc) fn index(
    krate: CrateData,
    package_id: PackageId,
    diagnostics: &DiagnosticSink,
) -> (Crate, AnnotatedItems) {
    let mut import_path2id: HashMap<_, _> = krate
        .paths
        .iter()
        .filter_map(|(id, summary)| {
            // We only want types, no macros
            if matches!(summary.kind(), ItemKind::Macro | ItemKind::ProcDerive) {
                return None;
            }
            // We will index local items on our own.
            // We don't get them from `paths` because it may include private items
            // as well, and we don't have a way to figure out if an item is private
            // or not from the summary info.
            if summary.crate_id() == 0 {
                return None;
            }

            Some((summary.path().into_owned(), id.to_owned()))
        })
        .collect();

    let mut annotation_queue = BTreeSet::<QueueItem>::new();
    let mut import_index = ImportIndex::default();
    let mut external_re_exports = Default::default();
    index_local_types(
        &krate,
        &package_id,
        IndexSet::new(),
        vec![],
        &mut import_index,
        &mut external_re_exports,
        &mut annotation_queue,
        &krate.root_item_id,
        true,
        None,
        false,
        diagnostics,
    );

    import_path2id.reserve(import_index.items.len());
    for (id, entry) in import_index.items.iter() {
        for path in entry.public_paths.iter().chain(entry.private_paths.iter()) {
            if !import_path2id.contains_key(&path.0) {
                import_path2id.insert(path.0.clone(), id.to_owned());
            }
        }
    }

    let krate = Crate::new(
        CrateCore { package_id, krate },
        ImportPath2Id::Eager(EagerImportPath2Id(import_path2id)),
        external_re_exports,
        import_index,
    );

    let annotated_items = annotations::process_queue(annotation_queue, &krate, diagnostics);
    (krate, annotated_items)
}
