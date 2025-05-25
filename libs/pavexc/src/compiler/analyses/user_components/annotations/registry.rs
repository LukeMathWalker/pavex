use std::{
    collections::{BTreeMap, BTreeSet},
    ops::Index,
};

use ahash::HashMap;
use guppy::{PackageId, graph::PackageGraph};
use itertools::Itertools as _;
use pavex_bp_schema::{CreatedAt, CreatedBy};
use pavexc_attr_parser::{AnnotationKind, AnnotationProperties};
use rustdoc_types::{Enum, ItemEnum, Struct};

use crate::rustdoc::{Crate, CrateCollection, GlobalItemId};

use super::{DiagnosticSink, diagnostic::*, queue::QueueItem, sortable::SortableId};

#[derive(Default)]
/// Map each package to its set of annotated items.
pub struct AnnotationRegistry {
    package_id2items: HashMap<PackageId, AnnotatedItems>,
}

impl AnnotationRegistry {
    /// Extract annotated items from the documentation of the specified packages.
    ///
    /// # Panics
    ///
    /// Panics if [`CrateCollection`] doesn't already contain the JSON docs for each specified package.
    pub fn bootstrap<I>(
        &mut self,
        package_ids: I,
        collection: &CrateCollection,
        diagnostics: &mut DiagnosticSink,
    ) where
        I: Iterator<Item = PackageId>,
    {
        for id in package_ids {
            let items = Self::_process(id.clone(), collection, diagnostics);
            self.package_id2items.insert(id, items);
        }
    }

    /// Extract annotated items from the documentation of the specified package.
    ///
    /// # Panics
    ///
    /// Panics if [`CrateCollection`] doesn't already contain the JSON docs for the specified package.
    fn _process(
        package_id: PackageId,
        collection: &CrateCollection,
        diagnostics: &mut DiagnosticSink,
    ) -> AnnotatedItems {
        let mut items = AnnotatedItems::default();
        let Some(krate) = collection.get_crate_by_package_id(&package_id) else {
            unreachable!(
                "The JSON documentation for {} should have been computed at this point.",
                package_id.repr()
            )
        };
        // We use a BTreeSet to guarantee a deterministic processing order.
        let mut queue: BTreeSet<_> = krate
            .import_index
            .items
            .iter()
            .filter_map(|(id, entry)| entry.is_public().then_some(QueueItem::Standalone(*id)))
            .collect();

        while let Some(queue_item) = queue.pop_last() {
            match queue_item {
                QueueItem::Standalone(id) => {
                    let item = krate.get_item_by_local_type_id(&id);

                    // Enqueue other items for analysis.
                    if let ItemEnum::Struct(Struct { impls, .. })
                    | ItemEnum::Enum(Enum { impls, .. }) = &item.inner
                    {
                        queue.extend(impls.iter().map(|impl_id| QueueItem::Impl {
                            self_: id,
                            id: *impl_id,
                        }));
                    }

                    if !matches!(
                        &item.inner,
                        ItemEnum::Struct(Struct { .. })
                            | ItemEnum::Enum(Enum { .. })
                            | ItemEnum::Function(_)
                    ) {
                        // We don't care about other item kinds.
                        continue;
                    }
                    let annotation = match pavexc_attr_parser::parse(&item.attrs) {
                        Ok(Some(annotation)) => annotation,
                        Ok(None) => {
                            continue;
                        }
                        Err(e) => {
                            // TODO: Only report an error if it's a crate from the current workspace
                            invalid_diagnostic_attribute(e, item.as_ref(), diagnostics);
                            continue;
                        }
                    };
                    if check_item_compatibility(&annotation, &item, diagnostics).is_err() {
                        continue;
                    }

                    items.item_id2details.insert(
                        id.into(),
                        AnnotatedItem {
                            id,
                            properties: annotation,
                            impl_: None,
                        },
                    );
                }
                QueueItem::Impl { self_, id: impl_id } => {
                    // Enqueue other items for analysis.
                    let impl_item = krate.get_item_by_local_type_id(&impl_id);
                    let ItemEnum::Impl(impl_) = &impl_item.inner else {
                        continue;
                    };
                    queue.extend(impl_.items.iter().map(|&item_id| QueueItem::ImplItem {
                        self_,
                        id: item_id,
                        impl_: impl_id,
                    }));
                }
                QueueItem::ImplItem { self_, impl_, id } => {
                    let item = krate.get_item_by_local_type_id(&id);
                    // We only care about methods here.
                    let ItemEnum::Function(_) = &item.inner else {
                        continue;
                    };
                    let annotation = match pavexc_attr_parser::parse(&item.attrs) {
                        Ok(Some(annotation)) => annotation,
                        Ok(None) => {
                            continue;
                        }
                        Err(e) => {
                            // TODO: Only report an error if it's a crate from the current workspace
                            invalid_diagnostic_attribute(e, item.as_ref(), diagnostics);
                            continue;
                        }
                    };
                    if check_item_compatibility(&annotation, &item, diagnostics).is_err() {
                        continue;
                    }
                    items.item_id2details.insert(
                        id.into(),
                        AnnotatedItem {
                            id,
                            properties: annotation,
                            impl_: Some(ImplInfo { self_, impl_ }),
                        },
                    );
                }
            }
        }
        items
    }

    /// Retrieve the annotation associated with the given item, if any.
    pub fn annotation(&self, item_id: &GlobalItemId) -> Option<&AnnotatedItem> {
        let items = self.package_id2items.get(&item_id.package_id)?;
        items.get(item_id.rustdoc_item_id)
    }
}

impl Index<&PackageId> for AnnotationRegistry {
    type Output = AnnotatedItems;

    fn index(&self, index: &PackageId) -> &Self::Output {
        &self.package_id2items[index]
    }
}

/// Report an error if the parsed annotation isn't compatible with the item
/// it was attached to.
fn check_item_compatibility(
    annotation: &AnnotationProperties,
    item: &rustdoc_types::Item,
    diagnostics: &mut DiagnosticSink,
) -> Result<(), ()> {
    match annotation.kind() {
        AnnotationKind::PreProcessingMiddleware
        | AnnotationKind::PostProcessingMiddleware
        | AnnotationKind::WrappingMiddleware
        | AnnotationKind::Fallback
        | AnnotationKind::ErrorObserver
        | AnnotationKind::Constructor
        | AnnotationKind::Route
            if matches!(item.inner, ItemEnum::Function(_)) => {}
        AnnotationKind::Prebuilt | AnnotationKind::Config
            if matches!(item.inner, ItemEnum::Enum(_) | ItemEnum::Struct(_)) => {}
        AnnotationKind::PreProcessingMiddleware
        | AnnotationKind::PostProcessingMiddleware
        | AnnotationKind::WrappingMiddleware
        | AnnotationKind::Constructor
        | AnnotationKind::ErrorObserver
        | AnnotationKind::Fallback
        | AnnotationKind::Prebuilt
        | AnnotationKind::Route
        | AnnotationKind::Config => {
            // TODO: Only emit an error if it's a workspace package.
            unsupported_item_kind(annotation.attribute(), item, diagnostics);
            return Err(());
        }
    }
    Ok(())
}

/// All the annotated items for a given package.
#[derive(Default)]
pub struct AnnotatedItems {
    item_id2details: BTreeMap<SortableId, AnnotatedItem>,
}

impl AnnotatedItems {
    /// Iterate over the annotated items in this package.
    pub fn iter(&self) -> impl Iterator<Item = (rustdoc_types::Id, &AnnotatedItem)> {
        self.item_id2details.iter().map(|(id, item)| (id.0, item))
    }

    /// Get the annotation for a specific item, if any.
    pub fn get(&self, id: rustdoc_types::Id) -> Option<&AnnotatedItem> {
        self.item_id2details.get(&id.into())
    }
}

/// An item decorated with a Pavex annotation.
pub struct AnnotatedItem {
    /// The identifier of the annotated item.
    pub id: rustdoc_types::Id,
    /// The content of the parsed Pavex annotation.
    pub properties: AnnotationProperties,
    /// Information about the `impl` block the item belongs to, if any.
    pub impl_: Option<ImplInfo>,
}

impl AnnotatedItem {
    /// Returns the annotation location metadata.
    pub fn created_at(&self, krate: &Crate, graph: &PackageGraph) -> Option<CreatedAt> {
        let id = match &self.impl_ {
            None => self.id,
            Some(impl_info) => {
                // FIXME: The `impl` where this method is defined may not be within the same module
                // where `Self` is defined.
                // See https://rust-lang.zulipchat.com/#narrow/channel/266220-t-rustdoc/topic/Module.20items.20don't.20link.20to.20impls.20.5Brustdoc-json.5D
                // for a discussion on this issue.
                impl_info.self_
            }
        };
        let item = krate.get_item_by_local_type_id(&id);
        match &item.inner {
            ItemEnum::Struct(..) | ItemEnum::Enum(..) | ItemEnum::Function(..) => {
                let module_path = {
                    let fn_path = krate.import_index.items[&item.id]
                        .defined_at
                        .as_ref()
                        .expect("No `defined_at` in the import index for a struct/enum/function/method item.");
                    fn_path.iter().take(fn_path.len() - 1).join("::")
                };
                Some(CreatedAt {
                    package_name: krate.crate_name(),
                    package_version: krate.crate_version(graph).to_string(),
                    module_path,
                })
            }
            _ => None,
        }
    }

    /// The name of the macro that was used to attach this annotation.
    pub fn created_by(&self) -> CreatedBy {
        let name = match self.properties.kind() {
            AnnotationKind::PreProcessingMiddleware => "pre_process",
            AnnotationKind::PostProcessingMiddleware => "post_process",
            AnnotationKind::WrappingMiddleware => "wrap",
            AnnotationKind::Constructor => "constructor",
            AnnotationKind::Config => "config",
            AnnotationKind::ErrorObserver => "error_observer",
            AnnotationKind::Prebuilt => "prebuilt",
            AnnotationKind::Route => "route",
            AnnotationKind::Fallback => "fallback",
        };
        CreatedBy::macro_name(name)
    }
}

/// Information about the `impl` block the item belongs to, if any.
pub struct ImplInfo {
    /// The `id` of the `Self` type for this `impl` block.
    pub self_: rustdoc_types::Id,
    /// The `id` of the `impl` block that this item belongs to.
    pub impl_: rustdoc_types::Id,
}
