mod diagnostic;
mod items;
mod queue;

use super::Crate;
use crate::diagnostic::DiagnosticSink;
use diagnostic::*;
use pavex_bp_schema::CreatedAt;
use pavexc_attr_parser::{AnnotationKind, AnnotationProperties};
use rustdoc_types::{Enum, ItemEnum, Struct, Trait};
use std::collections::BTreeSet;

pub(crate) use diagnostic::invalid_diagnostic_attribute;
pub use items::{AnnotatedItem, AnnotatedItems, ImplInfo};
pub(crate) use queue::QueueItem;

/// Enough information to locate an annotated component in the
/// package where it was defined.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AnnotationCoordinates {
    pub id: String,
    pub created_at: CreatedAt,
}

/// Extract annotated items from the documentation of the specified package.
///
/// # Panics
///
/// Panics if [`CrateCollection`] doesn't already contain the JSON docs for the specified package.
pub(crate) fn process_queue(
    mut queue: BTreeSet<QueueItem>,
    krate: &Crate,
    diagnostics: &DiagnosticSink,
) -> AnnotatedItems {
    let mut items = AnnotatedItems::default();
    while let Some(queue_item) = queue.pop_last() {
        match queue_item {
            QueueItem::Standalone(id) => {
                let item = krate.get_item_by_local_type_id(&id);

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
                if !matches!(
                    &item.inner,
                    ItemEnum::Struct(Struct { .. })
                        | ItemEnum::Enum(Enum { .. })
                        | ItemEnum::Function(_)
                        | ItemEnum::Use(_)
                ) {
                    continue;
                }
                if check_item_compatibility(&annotation, &item, diagnostics).is_err() {
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
                    id_conflict(e, krate, diagnostics);
                }
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

                // Check that the `impl` block has been annotated with #[pavex::methods].
                let impl_item = krate.get_item_by_local_type_id(&impl_);
                match pavexc_attr_parser::parse(&impl_item.attrs) {
                    Ok(Some(AnnotationProperties::Methods)) => {}
                    Ok(_) => {
                        missing_methods_attribute(
                            annotation.kind(),
                            impl_item.as_ref(),
                            item.as_ref(),
                            diagnostics,
                        );
                        continue;
                    }
                    Err(e) => {
                        // TODO: Only report an error if it's a crate from the current workspace
                        invalid_diagnostic_attribute(e, item.as_ref(), diagnostics);
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
                    id_conflict(e, krate, diagnostics);
                }
            }
        }
    }
    items
}

/// Report an error if the parsed annotation isn't compatible with the item
/// it was attached to.
fn check_item_compatibility(
    annotation: &AnnotationProperties,
    item: &rustdoc_types::Item,
    diagnostics: &DiagnosticSink,
) -> Result<(), ()> {
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
            // TODO: Only emit an error if it's a workspace package.
            unsupported_item_kind(annotation.attribute(), item, diagnostics);
            return Err(());
        }
    }
    Ok(())
}
