//! Convert `rustdoc_types` items into `rustdoc_ir` types and callables.
//!
//! This crate sits between `rustdoc_types`/`rustdoc_processor` (input) and
//! `rustdoc_ir` (output), providing the core type-resolution logic with no
//! framework-specific dependencies.

mod errors;
mod free_fn;
mod method;
mod resolve_type;
mod type_def;

pub use errors::*;
pub use free_fn::resolve_free_function;
pub use method::rustdoc_method2callable;
pub use resolve_type::{TypeAliasResolution, resolve_type};
pub use type_def::{rustdoc_item_def2type, rustdoc_new_type_def2type, rustdoc_type_alias2type};

use ahash::HashMap;
use rustdoc_ir::Type;

/// Maps generic parameter names to their resolved types or lifetimes during type resolution.
///
/// Used to substitute generic parameters with concrete types when resolving
/// type aliases and generic instantiations.
#[derive(Default, Clone)]
pub struct GenericBindings {
    /// Mapping from lifetime parameter names to their resolved lifetime names.
    pub lifetimes: HashMap<String, String>,
    /// Mapping from type parameter names to their resolved types.
    pub types: HashMap<String, Type>,
    /// Mapping from const parameter names to their evaluated values.
    pub consts: HashMap<String, String>,
}

impl std::fmt::Debug for GenericBindings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GenericBindings {{ ")?;
        if !self.lifetimes.is_empty() {
            write!(f, "lifetimes: {{ ")?;
            for (name, value) in &self.lifetimes {
                writeln!(f, "{name} -> {value}, ")?;
            }
            write!(f, "}}, ")?;
        }
        if !self.types.is_empty() {
            write!(f, "types: {{ ")?;
            for (name, value) in &self.types {
                writeln!(f, "{} -> {}, ", name, value.display_for_error())?;
            }
            write!(f, "}}, ")?;
        }
        if !self.consts.is_empty() {
            write!(f, "consts: {{ ")?;
            for (name, value) in &self.consts {
                writeln!(f, "{name} -> {value}, ")?;
            }
            write!(f, "}}, ")?;
        }
        write!(f, "}}")
    }
}
