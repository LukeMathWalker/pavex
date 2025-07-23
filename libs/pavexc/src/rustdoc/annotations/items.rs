use std::collections::BTreeMap;

use super::super::SortableId;
use pavex_bp_schema::CreatedBy;
use pavexc_attr_parser::{AnnotationKind, AnnotationProperties};

/// All the annotated items for a given package.
#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AnnotatedItems {
    item_id2details: BTreeMap<SortableId, AnnotatedItem>,
    annotation_id2item_id: BTreeMap<String, rustdoc_types::Id>,
}

impl AnnotatedItems {
    /// Iterate over the annotated items in this package.
    pub fn iter(&self) -> impl Iterator<Item = (rustdoc_types::Id, &AnnotatedItem)> {
        self.item_id2details.iter().map(|(id, item)| (id.0, item))
    }

    /// Get the annotation for a specific item, if any.
    pub fn get_by_item_id(&self, id: rustdoc_types::Id) -> Option<&AnnotatedItem> {
        self.item_id2details.get(&id.into())
    }

    /// Get the annotation with a specific id, if any.
    pub fn get_by_annotation_id(&self, id: &str) -> Option<&AnnotatedItem> {
        let item_id = self.annotation_id2item_id.get(id)?;
        self.get_by_item_id(*item_id)
    }

    /// Insert an annotated item.
    pub fn insert(&mut self, id: rustdoc_types::Id, item: AnnotatedItem) -> Result<(), IdConflict> {
        let annotation_id = item.properties.id().map(|s| s.to_owned());
        self.item_id2details.insert(id.into(), item);
        let Some(annotation_id) = annotation_id else {
            return Ok(());
        };
        let previous = self.annotation_id2item_id.insert(annotation_id.clone(), id);
        if let Some(previous) = previous
            && previous != id
        // ^ This can happen for trait methods, when both the trait and `Self` are defined in the same crate.
        {
            Err(IdConflict {
                first: id,
                second: previous,
                annotation_id,
            })
        } else {
            Ok(())
        }
    }
}

pub struct IdConflict {
    pub first: rustdoc_types::Id,
    pub second: rustdoc_types::Id,
    pub annotation_id: String,
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
