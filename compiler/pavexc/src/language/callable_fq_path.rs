use std::fmt::{Display, Formatter, Write};

use anyhow::Context;
use bimap::BiHashMap;
use guppy::PackageId;

use crate::language::fq_path::{FQGenericArgument, FQPathType};

/// A fully-qualified path to a callable, with the callable kind
/// made explicit in the type system.
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum CallablePath {
    FreeFunction(FreeFunctionPath),
    InherentMethod(InherentMethodPath),
    TraitMethod(TraitMethodPath),
    StructLiteral(StructLiteralPath),
    EnumVariantConstructor(EnumVariantConstructorPath),
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct FreeFunctionPath {
    pub package_id: PackageId,
    /// Crate name as registered in the dependency graph.
    pub crate_name: String,
    /// Module path from crate root (excludes crate name and function name).
    pub module_path: Vec<String>,
    pub function_name: String,
    pub function_generics: Vec<FQGenericArgument>,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct InherentMethodPath {
    pub package_id: PackageId,
    pub crate_name: String,
    /// Module path from crate root (excludes crate name and type name).
    pub module_path: Vec<String>,
    pub type_name: String,
    pub type_generics: Vec<FQGenericArgument>,
    pub method_name: String,
    pub method_generics: Vec<FQGenericArgument>,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct TraitMethodPath {
    /// Package of the trait (not the Self type).
    pub package_id: PackageId,
    pub crate_name: String,
    /// Module path from crate root (excludes crate name and trait name).
    pub module_path: Vec<String>,
    pub trait_name: String,
    pub trait_generics: Vec<FQGenericArgument>,
    pub self_type: FQPathType,
    pub method_name: String,
    pub method_generics: Vec<FQGenericArgument>,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct StructLiteralPath {
    pub package_id: PackageId,
    pub crate_name: String,
    /// Module path from crate root (excludes crate name and type name).
    pub module_path: Vec<String>,
    pub type_name: String,
    pub type_generics: Vec<FQGenericArgument>,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct EnumVariantConstructorPath {
    pub package_id: PackageId,
    pub crate_name: String,
    /// Module path from crate root (excludes crate name and enum name).
    pub module_path: Vec<String>,
    pub enum_name: String,
    pub enum_generics: Vec<FQGenericArgument>,
    pub variant_name: String,
}

impl CallablePath {
    pub fn package_id(&self) -> &PackageId {
        match self {
            CallablePath::FreeFunction(p) => &p.package_id,
            CallablePath::InherentMethod(p) => &p.package_id,
            CallablePath::TraitMethod(p) => &p.package_id,
            CallablePath::StructLiteral(p) => &p.package_id,
            CallablePath::EnumVariantConstructor(p) => &p.package_id,
        }
    }

    /// Returns the name of the callable itself (function name, method name, or type name for struct literals).
    #[allow(unused)]
    pub fn callable_name(&self) -> &str {
        match self {
            CallablePath::FreeFunction(p) => &p.function_name,
            CallablePath::InherentMethod(p) => &p.method_name,
            CallablePath::TraitMethod(p) => &p.method_name,
            CallablePath::StructLiteral(p) => &p.type_name,
            CallablePath::EnumVariantConstructor(p) => &p.variant_name,
        }
    }

    /// Returns the owner name (type/trait) for methods, `None` for free functions and struct literals.
    #[allow(unused)]
    pub fn owner_name(&self) -> Option<&str> {
        match self {
            CallablePath::FreeFunction(_) => None,
            CallablePath::InherentMethod(p) => Some(&p.type_name),
            CallablePath::TraitMethod(p) => Some(&p.trait_name),
            CallablePath::StructLiteral(_) => None,
            CallablePath::EnumVariantConstructor(p) => Some(&p.enum_name),
        }
    }

    pub fn render_path(&self, id2name: &BiHashMap<PackageId, String>, buffer: &mut String) {
        match self {
            CallablePath::FreeFunction(p) => p.render_path(id2name, buffer),
            CallablePath::InherentMethod(p) => p.render_path(id2name, buffer),
            CallablePath::TraitMethod(p) => p.render_path(id2name, buffer),
            CallablePath::StructLiteral(p) => p.render_path(id2name, buffer),
            CallablePath::EnumVariantConstructor(p) => p.render_path(id2name, buffer),
        }
    }

    pub fn render_for_error(&self, buffer: &mut String) {
        match self {
            CallablePath::FreeFunction(p) => p.render_for_error(buffer),
            CallablePath::InherentMethod(p) => p.render_for_error(buffer),
            CallablePath::TraitMethod(p) => p.render_for_error(buffer),
            CallablePath::StructLiteral(p) => p.render_for_error(buffer),
            CallablePath::EnumVariantConstructor(p) => p.render_for_error(buffer),
        }
    }
}

impl Display for CallablePath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CallablePath::FreeFunction(p) => write!(f, "{p}"),
            CallablePath::InherentMethod(p) => write!(f, "{p}"),
            CallablePath::TraitMethod(p) => write!(f, "{p}"),
            CallablePath::StructLiteral(p) => write!(f, "{p}"),
            CallablePath::EnumVariantConstructor(p) => write!(f, "{p}"),
        }
    }
}

// --- Helper: render generics ---

fn render_generics_for_codegen(
    generics: &[FQGenericArgument],
    id2name: &BiHashMap<PackageId, String>,
    buffer: &mut String,
) {
    if !generics.is_empty() {
        write!(buffer, "::<").unwrap();
        let mut args = generics.iter().peekable();
        while let Some(arg) = args.next() {
            arg.render_path(id2name, buffer);
            if args.peek().is_some() {
                write!(buffer, ", ").unwrap();
            }
        }
        write!(buffer, ">").unwrap();
    }
}

fn render_generics_for_error(generics: &[FQGenericArgument], buffer: &mut String) {
    if !generics.is_empty() {
        write!(buffer, "::<").unwrap();
        let mut args = generics.iter().peekable();
        while let Some(arg) = args.next() {
            arg.render_for_error(buffer);
            if args.peek().is_some() {
                write!(buffer, ", ").unwrap();
            }
        }
        write!(buffer, ">").unwrap();
    }
}

/// Write generics using `Display` (preserves named lifetimes like `'server`, `'request`).
fn write_generics_display(
    generics: &[FQGenericArgument],
    f: &mut Formatter<'_>,
) -> std::fmt::Result {
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
        self.self_type.render_path(id2name, buffer);
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
        self.self_type.render_for_error(buffer);
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
