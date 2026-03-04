mod diagnostic;

use super::Crate;
use crate::diagnostic::DiagnosticSink;
use pavexc_annotations::{AnnotationError, ItemProvider};
use pavexc_attr_parser::errors::AttributeParserError;
use rustdoc_types::Item;
use std::borrow::Cow;
use std::collections::BTreeSet;

pub(crate) use pavexc_annotations::parse_pavex_attributes;
pub use pavexc_annotations::{
    AnnotatedItem, AnnotatedItems, AnnotationCoordinates, ImplInfo, QueueItem,
};

/// Report an error for an invalid diagnostic attribute on an item.
pub(crate) fn invalid_diagnostic_attribute(
    e: AttributeParserError,
    item: &Item,
    diagnostics: &DiagnosticSink,
) {
    diagnostic::invalid_diagnostic_attribute(
        e,
        item.name.as_deref(),
        item.span.as_ref(),
        diagnostics,
    );
}

/// Adapter that implements `ItemProvider` for our `Crate` type.
struct CrateItemProvider<'a> {
    krate: &'a Crate,
}

impl ItemProvider for CrateItemProvider<'_> {
    fn get_item(&self, id: &rustdoc_types::Id) -> Cow<'_, rustdoc_types::Item> {
        self.krate.get_item_by_local_type_id(id)
    }

    fn maybe_get_item(&self, id: &rustdoc_types::Id) -> Option<Cow<'_, rustdoc_types::Item>> {
        self.krate.maybe_get_item_by_local_type_id(id)
    }
}

/// Extract annotated items from the documentation of the specified package.
///
/// # Panics
///
/// Panics if [`CrateCollection`] doesn't already contain the JSON docs for the specified package.
pub(crate) fn process_queue(
    queue: BTreeSet<QueueItem>,
    krate: &Crate,
    diagnostics: &DiagnosticSink,
) -> AnnotatedItems {
    let provider = CrateItemProvider { krate };
    let (items, errors) = pavexc_annotations::process_queue(queue, &provider);

    // Convert errors to diagnostics
    for error in errors {
        emit_annotation_error(error, krate, diagnostics);
    }

    items
}

/// Convert an `AnnotationError` into a diagnostic and emit it.
fn emit_annotation_error(error: AnnotationError, krate: &Crate, diagnostics: &DiagnosticSink) {
    match error {
        AnnotationError::InvalidAttribute {
            error,
            item_name,
            item_span,
        } => {
            diagnostic::invalid_diagnostic_attribute(
                error,
                item_name.as_deref(),
                item_span.as_ref(),
                diagnostics,
            );
        }
        AnnotationError::UnsupportedItemKind {
            attribute,
            item_name,
            item_kind,
            item_span,
        } => {
            diagnostic::unsupported_item_kind(
                &attribute,
                item_name.as_deref(),
                item_kind,
                item_span.as_ref(),
                diagnostics,
            );
        }
        AnnotationError::MissingMethodsAttribute {
            annotation_kind,
            item_name,
            item_span,
            impl_span,
        } => {
            diagnostic::missing_methods_attribute(
                annotation_kind,
                item_name.as_deref(),
                item_span.as_ref(),
                impl_span.as_ref(),
                diagnostics,
            );
        }
        AnnotationError::IdConflict {
            conflict,
            first_span,
            second_span,
        } => {
            diagnostic::id_conflict(
                conflict,
                first_span.as_ref(),
                second_span.as_ref(),
                krate,
                diagnostics,
            );
        }
    }
}
