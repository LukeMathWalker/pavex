//! Extension traits for types in `rustdoc_types`.
//!
//! This crate provides utility traits that augment the types from our fork of `rustdoc-types`.
//! We keep these extensions separate to maintain `rustdoc_types` as close to upstream
//! as possible.

mod global_item_id;
pub use global_item_id::GlobalItemId;

use rustdoc_types::{ItemEnum, ItemKind, MacroKind};

/// Extension trait for `ItemEnum` to get the corresponding `ItemKind`.
pub trait ItemEnumExt {
    /// Returns the `ItemKind` corresponding to this `ItemEnum` variant.
    fn item_kind(&self) -> ItemKind;
}

impl ItemEnumExt for ItemEnum {
    fn item_kind(&self) -> ItemKind {
        match self {
            ItemEnum::Module(_) => ItemKind::Module,
            ItemEnum::ExternCrate { .. } => ItemKind::ExternCrate,
            ItemEnum::Use(_) => ItemKind::Use,
            ItemEnum::Union(_) => ItemKind::Union,
            ItemEnum::Struct(_) => ItemKind::Struct,
            ItemEnum::StructField(_) => ItemKind::StructField,
            ItemEnum::Enum(_) => ItemKind::Enum,
            ItemEnum::Variant(_) => ItemKind::Variant,
            ItemEnum::Function(_) => ItemKind::Function,
            ItemEnum::Trait(_) => ItemKind::Trait,
            ItemEnum::TraitAlias(_) => ItemKind::TraitAlias,
            ItemEnum::Impl(_) => ItemKind::Impl,
            ItemEnum::TypeAlias(_) => ItemKind::TypeAlias,
            ItemEnum::Constant { .. } => ItemKind::Constant,
            ItemEnum::Static(_) => ItemKind::Static,
            ItemEnum::ExternType => ItemKind::ExternType,
            ItemEnum::Macro(_) => ItemKind::Macro,
            ItemEnum::ProcMacro(pm) => match pm.kind {
                MacroKind::Bang => ItemKind::Macro,
                MacroKind::Attr => ItemKind::ProcAttribute,
                MacroKind::Derive => ItemKind::ProcDerive,
            },
            ItemEnum::Primitive(_) => ItemKind::Primitive,
            ItemEnum::AssocConst { .. } => ItemKind::AssocConst,
            ItemEnum::AssocType { .. } => ItemKind::AssocType,
        }
    }
}

/// Extension trait for `ItemEnum` to get a human-readable description of the item kind.
pub trait RustdocKindExt {
    /// Return a human-readable string description of this item's kind (e.g. "a function").
    fn kind(&self) -> &'static str;
}

impl RustdocKindExt for ItemEnum {
    fn kind(&self) -> &'static str {
        match self {
            ItemEnum::Module(_) => "a module",
            ItemEnum::ExternCrate { .. } => "an external crate",
            ItemEnum::Use(_) => "an import",
            ItemEnum::Union(_) => "a union",
            ItemEnum::Struct(_) => "a struct",
            ItemEnum::StructField(_) => "a struct field",
            ItemEnum::Enum(_) => "an enum",
            ItemEnum::Variant(_) => "an enum variant",
            ItemEnum::Function(func) => {
                if let Some((param, _)) = func.sig.inputs.first()
                    && param == "self"
                {
                    "a method"
                } else {
                    "a function"
                }
            }
            ItemEnum::Trait(_) => "a trait",
            ItemEnum::TraitAlias(_) => "a trait alias",
            ItemEnum::Impl(_) => "an impl block",
            ItemEnum::TypeAlias(_) => "a type alias",
            ItemEnum::Constant { .. } => "a constant",
            ItemEnum::Static(_) => "a static",
            ItemEnum::ExternType => "a foreign type",
            ItemEnum::Macro(_) => "a macro",
            ItemEnum::ProcMacro(_) => "a procedural macro",
            ItemEnum::Primitive(_) => "a primitive type",
            ItemEnum::AssocConst { .. } => "an associated constant",
            ItemEnum::AssocType { .. } => "an associated type",
        }
    }
}

/// Extension trait for `ItemKind` to get human-readable descriptions.
pub trait ItemKindExt {
    /// Return the plural form of this item kind (e.g. "functions", "structs").
    fn plural(&self) -> &'static str;
}

impl ItemKindExt for ItemKind {
    fn plural(&self) -> &'static str {
        match self {
            ItemKind::Module => "modules",
            ItemKind::ExternCrate => "extern crate declarations",
            ItemKind::Use => "use declarations",
            ItemKind::Struct => "structs",
            ItemKind::StructField => "struct fields",
            ItemKind::Union => "unions",
            ItemKind::Enum => "enums",
            ItemKind::Variant => "enum variants",
            ItemKind::Function => "functions",
            ItemKind::TypeAlias => "type aliases",
            ItemKind::Constant => "constants",
            ItemKind::Trait => "traits",
            ItemKind::TraitAlias => "trait aliases",
            ItemKind::Impl => "impl blocks",
            ItemKind::Static => "statics",
            ItemKind::ExternType => "extern types",
            ItemKind::Macro => "macros",
            ItemKind::ProcAttribute => "proc macro attributes",
            ItemKind::ProcDerive => "derive macros",
            ItemKind::AssocConst => "associated constants",
            ItemKind::AssocType => "associated types",
            ItemKind::Primitive => "primitive types",
            ItemKind::Keyword => "keywords",
            ItemKind::Attribute => "attributes",
        }
    }
}
