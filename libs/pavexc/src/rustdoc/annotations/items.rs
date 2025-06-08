use std::collections::BTreeMap;

use guppy::graph::PackageGraph;
use itertools::Itertools as _;
use pavex_bp_schema::{CreatedAt, CreatedBy};
use pavexc_attr_parser::{AnnotationKind, AnnotationProperties};
use rustdoc_types::ItemEnum;

use super::super::{Crate, SortableId};

/// All the annotated items for a given package.
#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
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

    /// Insert an annotated item.
    pub fn insert(&mut self, id: rustdoc_types::Id, item: AnnotatedItem) {
        self.item_id2details.insert(id.into(), item);
    }
}

/// An item decorated with a Pavex annotation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
                impl_info.attached_to
            }
        };
        let item = krate.get_item_by_local_type_id(&id);
        match &item.inner {
            ItemEnum::Struct(..)
            | ItemEnum::Enum(..)
            | ItemEnum::Function(..)
            | ItemEnum::Trait(..) => {
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
            AnnotationKind::ErrorHandler => "error_handler",
            AnnotationKind::Prebuilt => "prebuilt",
            AnnotationKind::Route => "route",
            AnnotationKind::Fallback => "fallback",
            AnnotationKind::Methods => "methods",
        };
        CreatedBy::macro_name(name)
    }
}

/// Information about the `impl` block the item belongs to, if any.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImplInfo {
    /// The `id` of the item this `impl` block was attached to.
    /// For inherent methods, that's the `Self` type.
    /// For trait methods, it can either be `Self` or the trait itself.
    pub attached_to: rustdoc_types::Id,
    /// The `id` of the `impl` block that this item belongs to.
    pub impl_: rustdoc_types::Id,
}
