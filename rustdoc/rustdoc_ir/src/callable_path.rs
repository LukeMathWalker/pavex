use std::fmt::{Display, Formatter, Write};

use anyhow::Context;
use bimap::BiHashMap;
use guppy::PackageId;

use crate::{GenericArgument, Type};

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct FreeFunctionPath {
    pub package_id: PackageId,
    /// Crate name as registered in the dependency graph.
    pub crate_name: String,
    /// Module path from crate root (excludes crate name and function name).
    pub module_path: Vec<String>,
    pub function_name: String,
    pub function_generics: Vec<GenericArgument>,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct InherentMethodPath {
    pub package_id: PackageId,
    pub crate_name: String,
    /// Module path from crate root (excludes crate name and type name).
    pub module_path: Vec<String>,
    pub type_name: String,
    pub type_generics: Vec<GenericArgument>,
    pub method_name: String,
    pub method_generics: Vec<GenericArgument>,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct TraitMethodPath {
    /// Package of the trait (not the Self type).
    pub package_id: PackageId,
    pub crate_name: String,
    /// Module path from crate root (excludes crate name and trait name).
    pub module_path: Vec<String>,
    pub trait_name: String,
    pub trait_generics: Vec<GenericArgument>,
    pub self_type: Type,
    pub method_name: String,
    pub method_generics: Vec<GenericArgument>,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct StructLiteralPath {
    pub package_id: PackageId,
    pub crate_name: String,
    /// Module path from crate root (excludes crate name and type name).
    pub module_path: Vec<String>,
    pub type_name: String,
    pub type_generics: Vec<GenericArgument>,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct EnumVariantConstructorPath {
    pub package_id: PackageId,
    pub crate_name: String,
    /// Module path from crate root (excludes crate name and enum name).
    pub module_path: Vec<String>,
    pub enum_name: String,
    pub enum_generics: Vec<GenericArgument>,
    pub variant_name: String,
}

// --- Helper: render generics ---

fn render_generics_for_codegen(
    generics: &[GenericArgument],
    id2name: &BiHashMap<PackageId, String>,
    buffer: &mut String,
) {
    if !generics.is_empty() {
        write!(buffer, "::<").unwrap();
        let mut args = generics.iter().peekable();
        while let Some(arg) = args.next() {
            arg.render_with_inferred_lifetimes_into(id2name, buffer);
            if args.peek().is_some() {
                write!(buffer, ", ").unwrap();
            }
        }
        write!(buffer, ">").unwrap();
    }
}

fn render_generics_for_error(generics: &[GenericArgument], buffer: &mut String) {
    if !generics.is_empty() {
        write!(buffer, "::<").unwrap();
        let mut args = generics.iter().peekable();
        while let Some(arg) = args.next() {
            arg.display_for_error_into(buffer);
            if args.peek().is_some() {
                write!(buffer, ", ").unwrap();
            }
        }
        write!(buffer, ">").unwrap();
    }
}

/// Write generics using `Display` (preserves named lifetimes like `'server`, `'request`).
fn write_generics_display(generics: &[GenericArgument], f: &mut Formatter<'_>) -> std::fmt::Result {
    if !generics.is_empty() {
        write!(f, "::<")?;
        for (i, arg) in generics.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{arg}")?;
        }
        write!(f, ">")?;
    }
    Ok(())
}

// --- FreeFunctionPath ---

impl FreeFunctionPath {
    pub fn render_path(&self, id2name: &BiHashMap<PackageId, String>, buffer: &mut String) {
        let crate_name = id2name
            .get_by_left(&self.package_id)
            .with_context(|| {
                format!(
                    "The package id '{}' is missing from the id<>name mapping for crates.",
                    self.package_id
                )
            })
            .unwrap();
        write!(buffer, "{crate_name}").unwrap();
        for module in &self.module_path {
            write!(buffer, "::{module}").unwrap();
        }
        write!(buffer, "::{}", self.function_name).unwrap();
        render_generics_for_codegen(&self.function_generics, id2name, buffer);
    }

    pub fn render_for_error(&self, buffer: &mut String) {
        buffer.push_str(&self.crate_name);
        for module in &self.module_path {
            write!(buffer, "::{module}").unwrap();
        }
        write!(buffer, "::{}", self.function_name).unwrap();
        render_generics_for_error(&self.function_generics, buffer);
    }
}

impl Display for FreeFunctionPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.crate_name)?;
        for module in &self.module_path {
            write!(f, "::{module}")?;
        }
        write!(f, "::{}", self.function_name)?;
        write_generics_display(&self.function_generics, f)
    }
}

// --- InherentMethodPath ---

impl InherentMethodPath {
    pub fn render_path(&self, id2name: &BiHashMap<PackageId, String>, buffer: &mut String) {
        let crate_name = id2name
            .get_by_left(&self.package_id)
            .with_context(|| {
                format!(
                    "The package id '{}' is missing from the id<>name mapping for crates.",
                    self.package_id
                )
            })
            .unwrap();
        write!(buffer, "{crate_name}").unwrap();
        for module in &self.module_path {
            write!(buffer, "::{module}").unwrap();
        }
        write!(buffer, "::{}", self.type_name).unwrap();
        render_generics_for_codegen(&self.type_generics, id2name, buffer);
        write!(buffer, "::{}", self.method_name).unwrap();
        render_generics_for_codegen(&self.method_generics, id2name, buffer);
    }

    pub fn render_for_error(&self, buffer: &mut String) {
        buffer.push_str(&self.crate_name);
        for module in &self.module_path {
            write!(buffer, "::{module}").unwrap();
        }
        write!(buffer, "::{}", self.type_name).unwrap();
        render_generics_for_error(&self.type_generics, buffer);
        write!(buffer, "::{}", self.method_name).unwrap();
        render_generics_for_error(&self.method_generics, buffer);
    }
}

impl Display for InherentMethodPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.crate_name)?;
        for module in &self.module_path {
            write!(f, "::{module}")?;
        }
        write!(f, "::{}", self.type_name)?;
        write_generics_display(&self.type_generics, f)?;
        write!(f, "::{}", self.method_name)?;
        write_generics_display(&self.method_generics, f)
    }
}

// --- TraitMethodPath ---

impl TraitMethodPath {
    pub fn render_path(&self, id2name: &BiHashMap<PackageId, String>, buffer: &mut String) {
        let crate_name = id2name
            .get_by_left(&self.package_id)
            .with_context(|| {
                format!(
                    "The package id '{}' is missing from the id<>name mapping for crates.",
                    self.package_id
                )
            })
            .unwrap();
        write!(buffer, "<").unwrap();
        self.self_type
            .render_with_inferred_lifetimes_into(id2name, buffer);
        write!(buffer, " as {crate_name}").unwrap();
        for module in &self.module_path {
            write!(buffer, "::{module}").unwrap();
        }
        write!(buffer, "::{}", self.trait_name).unwrap();
        render_generics_for_codegen(&self.trait_generics, id2name, buffer);
        write!(buffer, ">").unwrap();
        write!(buffer, "::{}", self.method_name).unwrap();
        render_generics_for_codegen(&self.method_generics, id2name, buffer);
    }

    pub fn render_for_error(&self, buffer: &mut String) {
        write!(buffer, "<").unwrap();
        self.self_type.display_for_error_into(buffer);
        write!(buffer, " as {}", self.crate_name).unwrap();
        for module in &self.module_path {
            write!(buffer, "::{module}").unwrap();
        }
        write!(buffer, "::{}", self.trait_name).unwrap();
        render_generics_for_error(&self.trait_generics, buffer);
        write!(buffer, ">").unwrap();
        write!(buffer, "::{}", self.method_name).unwrap();
        render_generics_for_error(&self.method_generics, buffer);
    }
}

impl Display for TraitMethodPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "<")?;
        write!(f, "{}", self.self_type)?;
        write!(f, " as {}", self.crate_name)?;
        for module in &self.module_path {
            write!(f, "::{module}")?;
        }
        write!(f, "::{}", self.trait_name)?;
        write_generics_display(&self.trait_generics, f)?;
        write!(f, ">")?;
        write!(f, "::{}", self.method_name)?;
        write_generics_display(&self.method_generics, f)
    }
}

// --- StructLiteralPath ---

impl StructLiteralPath {
    pub fn render_path(&self, id2name: &BiHashMap<PackageId, String>, buffer: &mut String) {
        let crate_name = id2name
            .get_by_left(&self.package_id)
            .with_context(|| {
                format!(
                    "The package id '{}' is missing from the id<>name mapping for crates.",
                    self.package_id
                )
            })
            .unwrap();
        write!(buffer, "{crate_name}").unwrap();
        for module in &self.module_path {
            write!(buffer, "::{module}").unwrap();
        }
        write!(buffer, "::{}", self.type_name).unwrap();
        render_generics_for_codegen(&self.type_generics, id2name, buffer);
    }

    pub fn render_for_error(&self, buffer: &mut String) {
        buffer.push_str(&self.crate_name);
        for module in &self.module_path {
            write!(buffer, "::{module}").unwrap();
        }
        write!(buffer, "::{}", self.type_name).unwrap();
        render_generics_for_error(&self.type_generics, buffer);
    }
}

impl Display for StructLiteralPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.crate_name)?;
        for module in &self.module_path {
            write!(f, "::{module}")?;
        }
        write!(f, "::{}", self.type_name)?;
        write_generics_display(&self.type_generics, f)
    }
}

// --- EnumVariantConstructorPath ---

impl EnumVariantConstructorPath {
    pub fn render_path(&self, id2name: &BiHashMap<PackageId, String>, buffer: &mut String) {
        let crate_name = id2name
            .get_by_left(&self.package_id)
            .with_context(|| {
                format!(
                    "The package id '{}' is missing from the id<>name mapping for crates.",
                    self.package_id
                )
            })
            .unwrap();
        write!(buffer, "{crate_name}").unwrap();
        for module in &self.module_path {
            write!(buffer, "::{module}").unwrap();
        }
        write!(buffer, "::{}", self.enum_name).unwrap();
        render_generics_for_codegen(&self.enum_generics, id2name, buffer);
        write!(buffer, "::{}", self.variant_name).unwrap();
    }

    pub fn render_for_error(&self, buffer: &mut String) {
        buffer.push_str(&self.crate_name);
        for module in &self.module_path {
            write!(buffer, "::{module}").unwrap();
        }
        write!(buffer, "::{}", self.enum_name).unwrap();
        render_generics_for_error(&self.enum_generics, buffer);
        write!(buffer, "::{}", self.variant_name).unwrap();
    }
}

impl Display for EnumVariantConstructorPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.crate_name)?;
        for module in &self.module_path {
            write!(f, "::{module}")?;
        }
        write!(f, "::{}", self.enum_name)?;
        write_generics_display(&self.enum_generics, f)?;
        write!(f, "::{}", self.variant_name)
    }
}
