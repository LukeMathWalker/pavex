use std::collections::BTreeSet;

use guppy::PackageId;
use indexmap::IndexSet;
use rustdoc_types::{ItemEnum, Visibility};

use crate::diagnostic::DiagnosticSink;
use rustdoc_cache::{
    CrateData, EntryVisibility, ExternalReExports, ImportIndex, ImportIndexEntry,
};

use super::super::annotations::{QueueItem, invalid_diagnostic_attribute, parse_pavex_attributes};

pub(super) fn index_local_types<'a>(
    krate: &'a CrateData,
    package_id: &'a PackageId,
    // The ordered set of modules we navigated to reach this item.
    // It used to detect infinite loops.
    mut navigation_history: IndexSet<rustdoc_types::Id>,
    mut current_path: Vec<String>,
    import_index: &mut ImportIndex,
    re_exports: &mut ExternalReExports,
    annotation_queue: &mut BTreeSet<QueueItem>,
    current_item_id: &rustdoc_types::Id,
    is_public: bool,
    // Set when the current item has been re-exported via a `use` statement
    // that includes an `as` rename.
    renamed_to: Option<String>,
    // If `true`, we've encountered at least a `pub use`/`use` statement while
    // navigating to this item.
    encountered_use: bool,
    diagnostics: &DiagnosticSink,
) {
    // TODO: the way we handle `current_path` is extremely wasteful,
    //       we can likely reuse the same buffer throughout.
    let current_item = match krate.index.get(current_item_id) {
        None => {
            if let Some(summary) = krate.paths.get(current_item_id)
                && summary.kind == rustdoc_types::ItemKind::Primitive
            {
                // This is a known bug—see https://github.com/rust-lang/rust/issues/104064
                return;
            }
            panic!(
                "Failed to retrieve item id `{:?}` from the JSON `index` for package id `{}`.",
                &current_item_id,
                package_id.repr()
            )
        }
        Some(i) => i,
    };

    match parse_pavex_attributes(&current_item.attrs) {
        Ok(Some(_)) => {
            annotation_queue.insert(QueueItem::Standalone(*current_item_id));
        }
        Ok(None) => {}
        Err(e) => {
            // TODO: Only report an error if it's a crate from the current workspace
            invalid_diagnostic_attribute(e, current_item.as_ref(), diagnostics);
        }
    };

    let is_public = is_public && current_item.visibility == Visibility::Public;

    let mut add_to_import_index = |path: Vec<String>, is_module: bool| {
        let visibility = if is_public {
            EntryVisibility::Public
        } else {
            EntryVisibility::Private
        };
        let is_definition = !encountered_use;
        let index = if is_module {
            &mut import_index.modules
        } else {
            &mut import_index.items
        };
        match index.get_mut(current_item_id) {
            Some(entry) => {
                entry.insert(path.clone(), visibility);
                if is_definition {
                    entry.defined_at = Some(path);
                }
            }
            None => {
                index.insert(
                    *current_item_id,
                    ImportIndexEntry::new(path, visibility, is_definition),
                );
            }
        }
    };

    let current_item = current_item.as_ref();
    match &current_item.inner {
        ItemEnum::Module(m) => {
            let current_path_segment = renamed_to.unwrap_or_else(|| {
                current_item
                    .name
                    .as_deref()
                    .expect("All 'module' items have a 'name' property")
                    .to_owned()
            });
            current_path.push(current_path_segment);

            add_to_import_index(
                current_path
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>(),
                true,
            );

            navigation_history.insert(*current_item_id);
            for item_id in &m.items {
                index_local_types(
                    krate,
                    package_id,
                    navigation_history.clone(),
                    current_path.clone(),
                    import_index,
                    re_exports,
                    annotation_queue,
                    item_id,
                    is_public,
                    None,
                    encountered_use,
                    diagnostics,
                );
            }
        }
        ItemEnum::Use(i) => {
            let Some(imported_id) = &i.id else {
                return;
            };

            import_index
                .re_export2parent_module
                .insert(current_item.id, *navigation_history.last().unwrap());

            let Some(imported_item) = krate.index.get(imported_id) else {
                // We are looking at a public re-export of another crate
                // (e.g. `pub use hyper;`), one of its modules or one of its items.
                // Due to how re-exports are handled in `rustdoc`, the re-exported
                // items inside that foreign module will not be found in the `index`
                // for this crate.
                // We intentionally add foreign items to the index to get a "complete"
                // picture of all the types available in this crate.
                re_exports.insert(krate, current_item, &current_path);
                return;
            };
            if let ItemEnum::Module(re_exported_module) = &imported_item.inner {
                if !i.is_glob {
                    current_path.push(i.name.clone());
                }
                // In Rust it is possible to create infinite loops with local modules!
                // Minimal example:
                // ```rust
                // pub struct A;
                // mod inner {
                //   pub use crate as b;
                // }
                // ```
                // We use this check to detect if we're about to get stuck in an infinite
                // loop, so that we can break early.
                // It does mean that some paths that _would_ be valid won't be recognised,
                // but this pattern is rarely used and for the time being we don't want to
                // take the complexity hit of making visible paths lazily evaluated.
                let infinite_loop = !navigation_history.insert(*imported_id);
                if !infinite_loop {
                    for re_exported_item_id in &re_exported_module.items {
                        index_local_types(
                            krate,
                            package_id,
                            navigation_history.clone(),
                            current_path.clone(),
                            import_index,
                            re_exports,
                            annotation_queue,
                            re_exported_item_id,
                            is_public,
                            None,
                            true,
                            diagnostics,
                        );
                    }
                }
            } else {
                navigation_history.insert(*imported_id);

                if matches!(
                    imported_item.inner,
                    ItemEnum::Enum(_)
                        | ItemEnum::Struct(_)
                        | ItemEnum::Trait(_)
                        | ItemEnum::Function(_)
                        | ItemEnum::Primitive(_)
                        | ItemEnum::TypeAlias(_)
                ) {
                    // We keep track of the source path in our indexes.
                    // This is useful, in particular, if we don't have
                    // access to the source module of the imported item.
                    // This can happen when working with `std`/`alloc`/`core`
                    // since the JSON output doesn't include private/doc-hidden
                    // items.
                    let mut normalized_source_path = vec![];
                    let source_segments = i.source.split("::");
                    for segment in source_segments {
                        if segment == "self" {
                            normalized_source_path
                                .extend(current_path.iter().map(|s| s.to_string()));
                        } else if segment == "crate" {
                            normalized_source_path.push(current_path[0].to_string())
                        } else {
                            normalized_source_path.push(segment.to_string());
                        }
                    }
                    // Assume it's private unless we find out otherwise later on
                    match import_index.items.get_mut(imported_id) {
                        Some(entry) => {
                            entry.insert_private(normalized_source_path);
                        }
                        None => {
                            import_index.items.insert(
                                *imported_id,
                                ImportIndexEntry::new(
                                    normalized_source_path,
                                    EntryVisibility::Private,
                                    false,
                                ),
                            );
                        }
                    }
                }

                index_local_types(
                    krate,
                    package_id,
                    navigation_history,
                    current_path.clone(),
                    import_index,
                    re_exports,
                    annotation_queue,
                    imported_id,
                    is_public,
                    Some(i.name.clone()),
                    true,
                    diagnostics,
                );
            }
        }
        ItemEnum::Trait(_)
        | ItemEnum::Primitive(_)
        | ItemEnum::Function(_)
        | ItemEnum::Enum(_)
        | ItemEnum::Struct(_)
        | ItemEnum::TypeAlias(_) => {
            let name = current_item.name.as_deref().expect(
                "All 'struct', 'function', 'enum', 'type_alias', 'primitive' and 'trait' items have a 'name' property",
            );
            if matches!(current_item.inner, ItemEnum::Primitive(_)) {
                // E.g. `std::bool` won't work, `std::primitive::bool` does work but the `primitive` module
                // is not visible in the JSON docs for `std`/`core`.
                // A hacky workaround, but it works.
                current_path.push("primitive".into());
            }
            current_path.push(renamed_to.unwrap_or_else(|| name.to_owned()));
            let path: Vec<_> = current_path.into_iter().map(|s| s.to_string()).collect();
            add_to_import_index(path, false);

            // Even if the item itself may not be annotated, one of its impls may be.
            annotation_queue.insert(QueueItem::Standalone(*current_item_id));
        }
        _ => {}
    }
}
