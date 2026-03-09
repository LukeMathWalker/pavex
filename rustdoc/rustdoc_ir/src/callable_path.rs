use std::fmt::{Display, Formatter, Write};

use anyhow::Context;
use bimap::BiHashMap;
use guppy::PackageId;

use crate::render::{LifetimeStyle, PathStyle, RenderConfig};
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

/// Render a generic argument list (e.g. `::<A, B>`) into `buffer`.
///
/// Writes nothing if `generics` is empty.
fn render_generics<W: Write>(
    generics: &[GenericArgument],
    config: &RenderConfig<'_>,
    buffer: &mut W,
) {
    if !generics.is_empty() {
        write!(buffer, "::<").unwrap();
        let mut args = generics.iter().peekable();
        while let Some(arg) = args.next() {
            arg.render_into(config, buffer);
            if args.peek().is_some() {
                write!(buffer, ", ").unwrap();
            }
        }
        write!(buffer, ">").unwrap();
    }
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
        let config = RenderConfig {
            path: PathStyle::CrateLookup(id2name),
            lifetime: LifetimeStyle::Erase,
        };
        write!(buffer, "{crate_name}").unwrap();
        for module in &self.module_path {
            write!(buffer, "::{module}").unwrap();
        }
        write!(buffer, "::{}", self.function_name).unwrap();
        render_generics(&self.function_generics, &config, buffer);
    }

    pub fn render_for_error(&self, buffer: &mut String) {
        let config = RenderConfig {
            path: PathStyle::Direct,
            lifetime: LifetimeStyle::Preserve,
        };
        buffer.push_str(&self.crate_name);
        for module in &self.module_path {
            write!(buffer, "::{module}").unwrap();
        }
        write!(buffer, "::{}", self.function_name).unwrap();
        render_generics(&self.function_generics, &config, buffer);
    }
}

impl Display for FreeFunctionPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let config = RenderConfig {
            path: PathStyle::Direct,
            lifetime: LifetimeStyle::Preserve,
        };
        write!(f, "{}", self.crate_name)?;
        for module in &self.module_path {
            write!(f, "::{module}")?;
        }
        write!(f, "::{}", self.function_name)?;
        render_generics(&self.function_generics, &config, f);
        Ok(())
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
        let config = RenderConfig {
            path: PathStyle::CrateLookup(id2name),
            lifetime: LifetimeStyle::Erase,
        };
        write!(buffer, "{crate_name}").unwrap();
        for module in &self.module_path {
            write!(buffer, "::{module}").unwrap();
        }
        write!(buffer, "::{}", self.type_name).unwrap();
        render_generics(&self.type_generics, &config, buffer);
        write!(buffer, "::{}", self.method_name).unwrap();
        render_generics(&self.method_generics, &config, buffer);
    }

    pub fn render_for_error(&self, buffer: &mut String) {
        let config = RenderConfig {
            path: PathStyle::Direct,
            lifetime: LifetimeStyle::Preserve,
        };
        buffer.push_str(&self.crate_name);
        for module in &self.module_path {
            write!(buffer, "::{module}").unwrap();
        }
        write!(buffer, "::{}", self.type_name).unwrap();
        render_generics(&self.type_generics, &config, buffer);
        write!(buffer, "::{}", self.method_name).unwrap();
        render_generics(&self.method_generics, &config, buffer);
    }
}

impl Display for InherentMethodPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let config = RenderConfig {
            path: PathStyle::Direct,
            lifetime: LifetimeStyle::Preserve,
        };
        write!(f, "{}", self.crate_name)?;
        for module in &self.module_path {
            write!(f, "::{module}")?;
        }
        write!(f, "::{}", self.type_name)?;
        render_generics(&self.type_generics, &config, f);
        write!(f, "::{}", self.method_name)?;
        render_generics(&self.method_generics, &config, f);
        Ok(())
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
        let config = RenderConfig {
            path: PathStyle::CrateLookup(id2name),
            lifetime: LifetimeStyle::Erase,
        };
        write!(buffer, "<").unwrap();
        self.self_type.render_into(&config, buffer);
        write!(buffer, " as {crate_name}").unwrap();
        for module in &self.module_path {
            write!(buffer, "::{module}").unwrap();
        }
        write!(buffer, "::{}", self.trait_name).unwrap();
        render_generics(&self.trait_generics, &config, buffer);
        write!(buffer, ">").unwrap();
        write!(buffer, "::{}", self.method_name).unwrap();
        render_generics(&self.method_generics, &config, buffer);
    }

    pub fn render_for_error(&self, buffer: &mut String) {
        let config = RenderConfig {
            path: PathStyle::Direct,
            lifetime: LifetimeStyle::Preserve,
        };
        write!(buffer, "<").unwrap();
        self.self_type.render_into(&config, buffer);
        write!(buffer, " as {}", self.crate_name).unwrap();
        for module in &self.module_path {
            write!(buffer, "::{module}").unwrap();
        }
        write!(buffer, "::{}", self.trait_name).unwrap();
        render_generics(&self.trait_generics, &config, buffer);
        write!(buffer, ">").unwrap();
        write!(buffer, "::{}", self.method_name).unwrap();
        render_generics(&self.method_generics, &config, buffer);
    }
}

impl Display for TraitMethodPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let config = RenderConfig {
            path: PathStyle::Direct,
            lifetime: LifetimeStyle::Preserve,
        };
        write!(f, "<")?;
        self.self_type.render_into(&config, f);
        write!(f, " as {}", self.crate_name)?;
        for module in &self.module_path {
            write!(f, "::{module}")?;
        }
        write!(f, "::{}", self.trait_name)?;
        render_generics(&self.trait_generics, &config, f);
        write!(f, ">")?;
        write!(f, "::{}", self.method_name)?;
        render_generics(&self.method_generics, &config, f);
        Ok(())
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
        let config = RenderConfig {
            path: PathStyle::CrateLookup(id2name),
            lifetime: LifetimeStyle::Erase,
        };
        write!(buffer, "{crate_name}").unwrap();
        for module in &self.module_path {
            write!(buffer, "::{module}").unwrap();
        }
        write!(buffer, "::{}", self.type_name).unwrap();
        render_generics(&self.type_generics, &config, buffer);
    }

    pub fn render_for_error(&self, buffer: &mut String) {
        let config = RenderConfig {
            path: PathStyle::Direct,
            lifetime: LifetimeStyle::Preserve,
        };
        buffer.push_str(&self.crate_name);
        for module in &self.module_path {
            write!(buffer, "::{module}").unwrap();
        }
        write!(buffer, "::{}", self.type_name).unwrap();
        render_generics(&self.type_generics, &config, buffer);
    }
}

impl Display for StructLiteralPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let config = RenderConfig {
            path: PathStyle::Direct,
            lifetime: LifetimeStyle::Preserve,
        };
        write!(f, "{}", self.crate_name)?;
        for module in &self.module_path {
            write!(f, "::{module}")?;
        }
        write!(f, "::{}", self.type_name)?;
        render_generics(&self.type_generics, &config, f);
        Ok(())
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
        let config = RenderConfig {
            path: PathStyle::CrateLookup(id2name),
            lifetime: LifetimeStyle::Erase,
        };
        write!(buffer, "{crate_name}").unwrap();
        for module in &self.module_path {
            write!(buffer, "::{module}").unwrap();
        }
        write!(buffer, "::{}", self.enum_name).unwrap();
        render_generics(&self.enum_generics, &config, buffer);
        write!(buffer, "::{}", self.variant_name).unwrap();
    }

    pub fn render_for_error(&self, buffer: &mut String) {
        let config = RenderConfig {
            path: PathStyle::Direct,
            lifetime: LifetimeStyle::Preserve,
        };
        buffer.push_str(&self.crate_name);
        for module in &self.module_path {
            write!(buffer, "::{module}").unwrap();
        }
        write!(buffer, "::{}", self.enum_name).unwrap();
        render_generics(&self.enum_generics, &config, buffer);
        write!(buffer, "::{}", self.variant_name).unwrap();
    }
}

impl Display for EnumVariantConstructorPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let config = RenderConfig {
            path: PathStyle::Direct,
            lifetime: LifetimeStyle::Preserve,
        };
        write!(f, "{}", self.crate_name)?;
        for module in &self.module_path {
            write!(f, "::{module}")?;
        }
        write!(f, "::{}", self.enum_name)?;
        render_generics(&self.enum_generics, &config, f);
        write!(f, "::{}", self.variant_name)
    }
}
