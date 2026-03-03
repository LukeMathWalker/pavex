//! Annotation processing logic.

use std::borrow::Cow;
use std::collections::BTreeSet;

use rustdoc_ext::ItemEnumExt;
use pavexc_attr_parser::{AnnotationKind, AnnotationProperties};
use rustdoc_types::{Enum, Item, ItemEnum, Struct, Trait};

use crate::errors::AnnotationError;
use crate::parser::parse_pavex_attributes;
use crate::types::{AnnotatedItem, AnnotatedItems, ImplInfo, QueueItem};

/// Trait for providing access to crate items.
///
/// This allows the annotation processing to be decoupled from the specific
/// crate data structure used by the caller.
pub trait ItemProvider {
    /// Get an item by its ID. Panics if the item doesn't exist.
    fn get_item(&self, id: &rustdoc_types::Id) -> Cow<'_, Item>;

    /// Get an item by its ID, returning None if it doesn't exist.
    fn maybe_get_item(&self, id: &rustdoc_types::Id) -> Option<Cow<'_, Item>>;
}

/// Extract annotated items from the documentation of the specified package.
///
/// Returns the collected annotations and any errors that occurred during processing.
pub fn process_queue<P: ItemProvider>(
    mut queue: BTreeSet<QueueItem>,
    provider: &P,
) -> (AnnotatedItems, Vec<AnnotationError>) {
    let mut items = AnnotatedItems::default();
    let mut errors = Vec::new();

    while let Some(queue_item) = queue.pop_last() {
        match queue_item {
            QueueItem::Standalone(id) => {
                let item = provider.get_item(&id);

                // Enqueue other items for analysis.
                if let ItemEnum::Struct(Struct { impls, .. })
                | ItemEnum::Enum(Enum { impls, .. })
                | ItemEnum::Trait(Trait {
                    implementations: impls,
                    ..
                }) = &item.inner
                {
                    queue.extend(impls.iter().map(|impl_id| QueueItem::Impl {
                        self_: id,
                        id: *impl_id,
                    }));
                }

                let annotation = match parse_pavex_attributes(&item.attrs) {
                    Ok(Some(annotation)) => annotation,
                    Ok(None) => {
                        continue;
                    }
                    Err(e) => {
                        errors.push(AnnotationError::InvalidAttribute {
                            error: e,
                            item_name: item.name.clone(),
                            item_span: item.span.clone(),
                        });
                        continue;
                    }
                };
                if !matches!(
                    &item.inner,
                    ItemEnum::Struct(Struct { .. })
                        | ItemEnum::Enum(Enum { .. })
                        | ItemEnum::Function(_)
                        | ItemEnum::Use(_)
                ) {
                    continue;
                }
                if let Err(e) = check_item_compatibility(&annotation, &item) {
                    errors.push(e);
                    continue;
                }

                if let Err(e) = items.insert(
                    id,
                    AnnotatedItem {
                        id,
                        properties: annotation,
                        impl_: None,
                    },
                ) {
                    let first_span = provider.maybe_get_item(&e.first).and_then(|i| i.span.clone());
                    let second_span =
                        provider.maybe_get_item(&e.second).and_then(|i| i.span.clone());
                    errors.push(AnnotationError::IdConflict {
                        conflict: e,
                        first_span,
                        second_span,
                    });
                }
            }
            QueueItem::Impl { self_, id: impl_id } => {
                // Enqueue other items for analysis.
                let impl_item = provider.get_item(&impl_id);
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
                let item = provider.get_item(&id);
                // We only care about methods here.
                let ItemEnum::Function(_) = &item.inner else {
                    continue;
                };
                let annotation = match parse_pavex_attributes(&item.attrs) {
                    Ok(Some(annotation)) => annotation,
                    Ok(None) => {
                        continue;
                    }
                    Err(e) => {
                        errors.push(AnnotationError::InvalidAttribute {
                            error: e,
                            item_name: item.name.clone(),
                            item_span: item.span.clone(),
                        });
                        continue;
                    }
                };
                if let Err(e) = check_item_compatibility(&annotation, &item) {
                    errors.push(e);
                    continue;
                }

                // Check that the `impl` block has been annotated with #[pavex::methods].
                let impl_item = provider.get_item(&impl_);
                match parse_pavex_attributes(&impl_item.attrs) {
                    Ok(Some(AnnotationProperties::Methods)) => {}
                    Ok(_) => {
                        errors.push(AnnotationError::MissingMethodsAttribute {
                            annotation_kind: annotation.kind(),
                            item_name: item.name.clone(),
                            item_span: item.span.clone(),
                            impl_span: impl_item.span.clone(),
                        });
                        continue;
                    }
                    Err(e) => {
                        errors.push(AnnotationError::InvalidAttribute {
                            error: e,
                            item_name: item.name.clone(),
                            item_span: item.span.clone(),
                        });
                        continue;
                    }
                };

                if let Err(e) = items.insert(
                    id,
                    AnnotatedItem {
                        id,
                        properties: annotation,
                        impl_: Some(ImplInfo {
                            attached_to: self_,
                            impl_,
                        }),
                    },
                ) {
                    let first_span = provider.maybe_get_item(&e.first).and_then(|i| i.span.clone());
                    let second_span =
                        provider.maybe_get_item(&e.second).and_then(|i| i.span.clone());
                    errors.push(AnnotationError::IdConflict {
                        conflict: e,
                        first_span,
                        second_span,
                    });
                }
            }
        }
    }
    (items, errors)
}

/// Check if the parsed annotation is compatible with the item it was attached to.
fn check_item_compatibility(
    annotation: &AnnotationProperties,
    item: &rustdoc_types::Item,
) -> Result<(), AnnotationError> {
    match annotation.kind() {
        AnnotationKind::PreProcessingMiddleware
        | AnnotationKind::PostProcessingMiddleware
        | AnnotationKind::WrappingMiddleware
        | AnnotationKind::Fallback
        | AnnotationKind::ErrorObserver
        | AnnotationKind::ErrorHandler
        | AnnotationKind::Constructor
        | AnnotationKind::Route
            if matches!(item.inner, ItemEnum::Function(_)) => {}
        AnnotationKind::Prebuilt | AnnotationKind::Config
            if matches!(
                item.inner,
                ItemEnum::Enum(_) | ItemEnum::Struct(_) | ItemEnum::Use(_)
            ) => {}
        AnnotationKind::PreProcessingMiddleware
        | AnnotationKind::PostProcessingMiddleware
        | AnnotationKind::WrappingMiddleware
        | AnnotationKind::Constructor
        | AnnotationKind::ErrorObserver
        | AnnotationKind::ErrorHandler
        | AnnotationKind::Fallback
        | AnnotationKind::Prebuilt
        | AnnotationKind::Route
        | AnnotationKind::Methods
        | AnnotationKind::Config => {
            return Err(AnnotationError::UnsupportedItemKind {
                attribute: annotation.attribute().to_owned(),
                item_name: item.name.clone(),
                item_kind: item.inner.item_kind(),
                item_span: item.span.clone(),
            });
        }
    }
    Ok(())
}
