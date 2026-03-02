//! Error types for annotation processing.

use pavexc_attr_parser::{AnnotationKind, errors::AttributeParserError};
use rustdoc_types::ItemKind;

use crate::IdConflict;

/// An error that occurred during annotation processing.
#[derive(Debug)]
pub enum AnnotationError {
    /// A Pavex attribute was malformed and couldn't be parsed.
    InvalidAttribute {
        /// The parsing error.
        error: AttributeParserError,
        /// The name of the item, if available.
        item_name: Option<String>,
        /// The span of the item in the source file.
        item_span: Option<rustdoc_types::Span>,
    },
    /// A Pavex annotation was attached to an item kind that doesn't support it.
    UnsupportedItemKind {
        /// The attribute that was used.
        attribute: String,
        /// The name of the item, if available.
        item_name: Option<String>,
        /// The kind of item the attribute was attached to.
        item_kind: ItemKind,
        /// The span of the item in the source file.
        item_span: Option<rustdoc_types::Span>,
    },
    /// A method was annotated with a Pavex attribute but the containing `impl` block
    /// is missing the `#[pavex::methods]` attribute.
    MissingMethodsAttribute {
        /// The kind of annotation on the method.
        annotation_kind: AnnotationKind,
        /// The name of the method, if available.
        item_name: Option<String>,
        /// The span of the method in the source file.
        item_span: Option<rustdoc_types::Span>,
        /// The span of the impl block in the source file.
        impl_span: Option<rustdoc_types::Span>,
    },
    /// Two items have the same annotation ID.
    IdConflict {
        /// The conflict details.
        conflict: IdConflict,
        /// The span of the first item, if available.
        first_span: Option<rustdoc_types::Span>,
        /// The span of the second item, if available.
        second_span: Option<rustdoc_types::Span>,
    },
}
