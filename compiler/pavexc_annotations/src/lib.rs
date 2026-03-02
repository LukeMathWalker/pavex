//! Pavex annotation processing.
//!
//! This crate provides functionality for extracting and processing Pavex annotations
//! from rustdoc JSON documentation.

mod errors;
mod parser;
mod process;
mod types;

pub use errors::AnnotationError;
pub use parser::parse_pavex_attributes;
pub use process::{ItemProvider, process_queue};
pub use types::{
    AnnotatedItem, AnnotatedItems, AnnotationCoordinates, IdConflict, ImplInfo, QueueItem,
};
