use std::fmt::Write;
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};

use anyhow::Context;
use bimap::BiHashMap;
use guppy::PackageId;
use itertools::Itertools;

use rustdoc_ir::function_pointer::write_fn_pointer_prefix;

use crate::language::Type;
use crate::language::resolved_type::{GenericArgument, Lifetime, ScalarPrimitive};

use super::resolved_type::{GenericLifetimeParameter, NamedLifetime};

/// A fully-qualified import path.
///
/// What does "fully qualified" mean in this contest?
///
/// `FQPath` ensures that all paths are "fully qualified"—i.e.
/// the first path segment is either the name of the current package or the name of a
/// crate listed as a dependency of the current package.
///
/// It also performs basic normalization.
/// There are ways, in Rust, to have different paths pointing at the same type.
///
/// E.g. `crate_name::TypeName` and `::crate_name::TypeName` are equivalent when `crate_name`
/// is a third-party dependency of your project. `ResolvedPath` reduces those two different
/// representations to a canonical one, allowing for deduplication.
///
/// Another common scenario: dependency renaming.
/// `crate_name::TypeName` and `renamed_crate_name::TypeName` can be equivalent if `crate_name`
/// has been renamed to `renamed_crate_name` in the `Cargo.toml` of the package that declares/uses
/// the path. `FQPath` takes this into account by using the `PackageId` of the target
/// crate as the authoritative answer to "What crate does this path belong to?". This is unique
/// and well-defined within a `cargo` workspace.
#[derive(Clone, Debug, Eq)]
pub struct FQPath {
    pub segments: Vec<FQPathSegment>,
    /// The qualified self of the path, if any.
    ///
    /// E.g. `Type` in `<Type as Trait>::Method`.
    pub qualified_self: Option<FQQualifiedSelf>,
    /// The package id of the crate that this path belongs to.
    ///
    /// For trait methods, it must be set to the package id of the crate where the
    /// trait is defined.
    pub package_id: PackageId,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct FQQualifiedSelf {
    pub position: usize,
    pub type_: FQPathType,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct FQPathSegment {
    pub ident: String,
    pub generic_arguments: Vec<FQGenericArgument>,
}

impl FQPathSegment {
    /// Create a new segment without generic arguments.
    pub fn new(ident: String) -> Self {
        Self {
            ident,
            generic_arguments: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum FQGenericArgument {
    Type(FQPathType),
    Lifetime(ResolvedPathLifetime),
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum ResolvedPathLifetime {
    Static,
    Named(NamedLifetime),
    Inferred,
}

impl ResolvedPathLifetime {
    /// Construct from a lifetime name, with or without the leading `'`.
    ///
    /// Routes `"_"` → `Inferred`, `"static"` → `Static`, everything else → `Named`.
    /// Returns the lifetime name without the leading `'`, suitable for use as a generic binding key.
    pub fn to_binding_name(&self) -> String {
        match self {
            ResolvedPathLifetime::Named(n) => n.as_str().to_owned(),
            ResolvedPathLifetime::Static => "static".to_owned(),
            ResolvedPathLifetime::Inferred => "_".to_owned(),
        }
    }

    /// Construct from a lifetime name, with or without the leading `'`.
    ///
    /// Routes `"_"` → `Inferred`, `"static"` → `Static`, everything else → `Named`.
    pub fn from_name(name: impl Into<String>) -> Self {
        let mut name = name.into();
        if let Some(stripped) = name.strip_prefix('\'') {
            name = stripped.to_owned();
        }
        match name.as_str() {
            "_" => ResolvedPathLifetime::Inferred,
            "static" => ResolvedPathLifetime::Static,
            _ => ResolvedPathLifetime::Named(NamedLifetime::new(name)),
        }
    }
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum FQPathType {
    ResolvedPath(FQResolvedPathType),
    Reference(FQReference),
    Tuple(FQTuple),
    ScalarPrimitive(ScalarPrimitive),
    Slice(FQSlice),
    Array(FQArray),
    RawPointer(FQRawPointer),
    FunctionPointer(FQFunctionPointer),
}

impl From<Type> for FQPathType {
    fn from(value: Type) -> Self {
        match value {
            Type::Path(p) => {
                let mut segments: Vec<FQPathSegment> = p
                    .base_type
                    .iter()
                    .map(|s| FQPathSegment {
                        ident: s.to_string(),
                        generic_arguments: vec![],
                    })
                    .collect();
                if let Some(segment) = segments.last_mut() {
                    segment.generic_arguments = p
                        .generic_arguments
                        .into_iter()
                        .map(|t| match t {
                            GenericArgument::TypeParameter(t) => FQGenericArgument::Type(t.into()),
                            GenericArgument::Lifetime(l) => FQGenericArgument::Lifetime(l.into()),
                        })
                        .collect();
                }
                FQPathType::ResolvedPath(FQResolvedPathType {
                    path: Box::new(FQPath {
                        segments,
                        qualified_self: None,
                        package_id: p.package_id,
                    }),
                })
            }
            Type::Reference(r) => FQPathType::Reference(FQReference {
                is_mutable: r.is_mutable,
                lifetime: r.lifetime,
                inner: Box::new((*r.inner).into()),
            }),
            Type::Tuple(t) => FQPathType::Tuple(FQTuple {
                elements: t.elements.into_iter().map(|e| e.into()).collect(),
            }),
            Type::ScalarPrimitive(s) => FQPathType::ScalarPrimitive(s),
            Type::Slice(s) => FQPathType::Slice(FQSlice {
                element: Box::new((*s.element_type).into()),
            }),
            Type::Array(a) => FQPathType::Array(FQArray {
                element: Box::new((*a.element_type).into()),
                len: a.len,
            }),
            Type::RawPointer(r) => FQPathType::RawPointer(FQRawPointer {
                is_mutable: r.is_mutable,
                inner: Box::new((*r.inner).into()),
            }),
            Type::FunctionPointer(fp) => FQPathType::FunctionPointer(FQFunctionPointer {
                inputs: fp.inputs.into_iter().map(|t| t.into()).collect(),
                output: fp.output.map(|t| Box::new((*t).into())),
                abi: fp.abi,
                is_unsafe: fp.is_unsafe,
            }),
            Type::Generic(_) => {
                // ResolvedPath doesn't support unassigned generic parameters.
                unreachable!("UnassignedGeneric")
            }
        }
    }
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct FQResolvedPathType {
    pub path: Box<FQPath>,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct FQReference {
    pub is_mutable: bool,
    pub lifetime: Lifetime,
    pub inner: Box<FQPathType>,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct FQTuple {
    pub elements: Vec<FQPathType>,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct FQSlice {
    pub element: Box<FQPathType>,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct FQArray {
    pub element: Box<FQPathType>,
    pub len: usize,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct FQRawPointer {
    pub is_mutable: bool,
    pub inner: Box<FQPathType>,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct FQFunctionPointer {
    pub inputs: Vec<FQPathType>,
    pub output: Option<Box<FQPathType>>,
    pub abi: rustdoc_types::Abi,
    pub is_unsafe: bool,
}

impl PartialEq for FQPath {
    fn eq(&self, other: &Self) -> bool {
        // Using destructuring syntax to make sure we get a compiler error
        // if a new field gets added, as a reminder to update this Hash implementation.
        let Self {
            segments,
            qualified_self,
            package_id,
        } = self;
        let Self {
            segments: other_segments,
            qualified_self: other_qualified_self,
            package_id: other_package_id,
        } = other;
        let is_equal = package_id == other_package_id
            && segments.len() == other_segments.len()
            && qualified_self == other_qualified_self;
        if is_equal {
            // We want to ignore the first segment of the path, because dependencies can be
            // renamed and this can lead to equivalent paths not being considered equal.
            // Given that we already have the package id as part of the type, it is safe
            // to disregard the first segment when determining equality.
            let self_segments = segments.iter().skip(1);
            let other_segments = other_segments.iter().skip(1);
            for (self_segment, other_segment) in self_segments.zip_eq(other_segments) {
                if self_segment != other_segment {
                    return false;
                }
            }
            true
        } else {
            false
        }
    }
}

impl Hash for FQPath {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Using destructuring syntax to make sure we get a compiler error
        // if a new field gets added, as a reminder to update this Hash implementation.
        let Self {
            segments,
            qualified_self,
            package_id,
        } = self;
        package_id.hash(state);
        qualified_self.hash(state);
        // We want to ignore the first segment of the path, because dependencies can be
        // renamed and this can lead to equivalent paths not being considered equal.
        // Given that we already have the package id as part of the type, it is safe
        // to disregard the first segment when determining equality.
        let self_segments = segments.iter().skip(1);
        for segment in self_segments {
            segment.hash(state)
        }
    }
}

impl FQPath {
    /// Return the name of the crate that this type belongs to.
    ///
    /// E.g. `my_crate::my_module::MyType` will return `my_crate`.
    pub fn crate_name(&self) -> &str {
        // This unwrap never fails thanks to the validation done in `parse`
        &self.segments.first().unwrap().ident
    }

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
        let mut qself_closing_wedge_index = None;
        if let Some(qself) = &self.qualified_self {
            write!(buffer, "<").unwrap();
            qself.type_.render_path(id2name, buffer);
            write!(buffer, " as ",).unwrap();
            qself_closing_wedge_index = Some(qself.position.saturating_sub(1));
        }
        write!(buffer, "{crate_name}").unwrap();
        for (index, path_segment) in self.segments[1..].iter().enumerate() {
            write!(buffer, "::{}", path_segment.ident).unwrap();
            let generic_arguments = &path_segment.generic_arguments;
            if !generic_arguments.is_empty() {
                write!(buffer, "::<").unwrap();
                let mut arguments = generic_arguments.iter().peekable();
                while let Some(argument) = arguments.next() {
                    argument.render_path(id2name, buffer);
                    if arguments.peek().is_some() {
                        write!(buffer, ", ").unwrap();
                    }
                }
                write!(buffer, ">").unwrap();
            }
            if Some(index + 1) == qself_closing_wedge_index {
                write!(buffer, ">").unwrap();
            }
        }
    }

    /// A utility method to render the path for usage in error messages.
    ///
    /// It doesn't require a "package_id <> name" mapping.
    pub fn render_for_error(&self, buffer: &mut String) {
        let mut qself_closing_wedge_index = None;
        if let Some(qself) = &self.qualified_self {
            write!(buffer, "<").unwrap();
            qself.type_.render_for_error(buffer);
            write!(buffer, " as ",).unwrap();
            qself_closing_wedge_index = Some(qself.position.saturating_sub(1));
        }
        for (index, path_segment) in self.segments.iter().enumerate() {
            if index != 0 {
                buffer.push_str("::");
            }
            buffer.push_str(&path_segment.ident);
            let generic_arguments = &path_segment.generic_arguments;
            if !generic_arguments.is_empty() {
                write!(buffer, "::<").unwrap();
                let mut arguments = generic_arguments.iter().peekable();
                while let Some(argument) = arguments.next() {
                    argument.render_for_error(buffer);
                    if arguments.peek().is_some() {
                        write!(buffer, ", ").unwrap();
                    }
                }
                write!(buffer, ">").unwrap();
            }
            if Some(index + 1) == qself_closing_wedge_index {
                write!(buffer, ">").unwrap();
            }
        }
    }
}

impl FQPathType {
    pub fn render_path(&self, id2name: &BiHashMap<PackageId, String>, buffer: &mut String) {
        match self {
            FQPathType::ResolvedPath(p) => p.render_path(id2name, buffer),
            FQPathType::Reference(r) => r.render_path(id2name, buffer),
            FQPathType::Tuple(t) => t.render_path(id2name, buffer),
            FQPathType::ScalarPrimitive(s) => {
                write!(buffer, "{s}").unwrap();
            }
            FQPathType::Slice(s) => s.render_path(id2name, buffer),
            FQPathType::Array(a) => a.render_path(id2name, buffer),
            FQPathType::RawPointer(r) => r.render_path(id2name, buffer),
            FQPathType::FunctionPointer(fp) => fp.render_path(id2name, buffer),
        }
    }

    pub fn render_for_error(&self, buffer: &mut String) {
        match self {
            FQPathType::ResolvedPath(p) => p.render_for_error(buffer),
            FQPathType::Reference(r) => r.render_for_error(buffer),
            FQPathType::Tuple(t) => t.render_for_error(buffer),
            FQPathType::ScalarPrimitive(s) => {
                write!(buffer, "{s}").unwrap();
            }
            FQPathType::Slice(s) => s.render_for_error(buffer),
            FQPathType::Array(a) => a.render_for_error(buffer),
            FQPathType::RawPointer(r) => r.render_for_error(buffer),
            FQPathType::FunctionPointer(fp) => fp.render_for_error(buffer),
        }
    }
}

impl FQSlice {
    pub fn render_path(&self, id2name: &BiHashMap<PackageId, String>, buffer: &mut String) {
        write!(buffer, "[").unwrap();
        self.element.render_path(id2name, buffer);
        write!(buffer, "]").unwrap();
    }

    pub fn render_for_error(&self, buffer: &mut String) {
        write!(buffer, "[").unwrap();
        self.element.render_for_error(buffer);
        write!(buffer, "]").unwrap();
    }
}

impl FQArray {
    pub fn render_path(&self, id2name: &BiHashMap<PackageId, String>, buffer: &mut String) {
        write!(buffer, "[").unwrap();
        self.element.render_path(id2name, buffer);
        write!(buffer, "; {}]", self.len).unwrap();
    }

    pub fn render_for_error(&self, buffer: &mut String) {
        write!(buffer, "[").unwrap();
        self.element.render_for_error(buffer);
        write!(buffer, "; {}]", self.len).unwrap();
    }
}

impl FQGenericArgument {
    pub fn render_path(&self, id2name: &BiHashMap<PackageId, String>, buffer: &mut String) {
        match self {
            FQGenericArgument::Type(t) => {
                t.render_path(id2name, buffer);
            }
            FQGenericArgument::Lifetime(l) => match l {
                ResolvedPathLifetime::Static => {
                    write!(buffer, "'static").unwrap();
                }
                ResolvedPathLifetime::Named(_) | ResolvedPathLifetime::Inferred => {
                    // TODO: we should have proper name mapping for lifetimes here, but we know all our current
                    //   usecases will work with lifetime elision.
                    write!(buffer, "'_").unwrap();
                }
            },
        }
    }

    pub fn render_for_error(&self, buffer: &mut String) {
        match self {
            FQGenericArgument::Type(t) => {
                t.render_for_error(buffer);
            }
            FQGenericArgument::Lifetime(l) => match l {
                ResolvedPathLifetime::Static => {
                    write!(buffer, "'static").unwrap();
                }
                ResolvedPathLifetime::Named(_) | ResolvedPathLifetime::Inferred => {
                    // TODO: we should have proper name mapping for lifetimes here, but we know all our current
                    //   usecases will work with lifetime elision.
                    write!(buffer, "'_").unwrap();
                }
            },
        }
    }
}

impl FQResolvedPathType {
    pub fn render_path(&self, id2name: &BiHashMap<PackageId, String>, buffer: &mut String) {
        self.path.render_path(id2name, buffer);
    }

    pub fn render_for_error(&self, buffer: &mut String) {
        self.path.render_for_error(buffer);
    }
}

impl FQTuple {
    pub fn render_path(&self, id2name: &BiHashMap<PackageId, String>, buffer: &mut String) {
        write!(buffer, "(").unwrap();
        let mut types = self.elements.iter().peekable();
        while let Some(ty) = types.next() {
            ty.render_path(id2name, buffer);
            if types.peek().is_some() {
                write!(buffer, ", ").unwrap();
            }
        }
        write!(buffer, ")").unwrap();
    }

    pub fn render_for_error(&self, buffer: &mut String) {
        write!(buffer, "(").unwrap();
        let mut types = self.elements.iter().peekable();
        while let Some(ty) = types.next() {
            ty.render_for_error(buffer);
            if types.peek().is_some() {
                write!(buffer, ", ").unwrap();
            }
        }
        write!(buffer, ")").unwrap();
    }
}

impl FQReference {
    pub fn render_path(&self, id2name: &BiHashMap<PackageId, String>, buffer: &mut String) {
        write!(buffer, "&").unwrap();
        if self.is_mutable {
            write!(buffer, "mut ").unwrap();
        }
        self.inner.render_path(id2name, buffer);
    }

    pub fn render_for_error(&self, buffer: &mut String) {
        write!(buffer, "&").unwrap();
        if self.is_mutable {
            write!(buffer, "mut ").unwrap();
        }
        self.inner.render_for_error(buffer);
    }
}

impl FQRawPointer {
    pub fn render_path(&self, id2name: &BiHashMap<PackageId, String>, buffer: &mut String) {
        if self.is_mutable {
            write!(buffer, "*mut ").unwrap();
        } else {
            write!(buffer, "*const ").unwrap();
        }
        self.inner.render_path(id2name, buffer);
    }

    pub fn render_for_error(&self, buffer: &mut String) {
        if self.is_mutable {
            write!(buffer, "*mut ").unwrap();
        } else {
            write!(buffer, "*const ").unwrap();
        }
        self.inner.render_for_error(buffer);
    }
}

impl FQFunctionPointer {
    pub fn render_path(&self, id2name: &BiHashMap<PackageId, String>, buffer: &mut String) {
        write_fn_pointer_prefix(buffer, &self.abi, self.is_unsafe).unwrap();
        write!(buffer, "fn(").unwrap();
        let mut inputs = self.inputs.iter().peekable();
        while let Some(input) = inputs.next() {
            input.render_path(id2name, buffer);
            if inputs.peek().is_some() {
                write!(buffer, ", ").unwrap();
            }
        }
        write!(buffer, ")").unwrap();
        if let Some(output) = &self.output {
            write!(buffer, " -> ").unwrap();
            output.render_path(id2name, buffer);
        }
    }

    pub fn render_for_error(&self, buffer: &mut String) {
        write_fn_pointer_prefix(buffer, &self.abi, self.is_unsafe).unwrap();
        write!(buffer, "fn(").unwrap();
        let mut inputs = self.inputs.iter().peekable();
        while let Some(input) = inputs.next() {
            input.render_for_error(buffer);
            if inputs.peek().is_some() {
                write!(buffer, ", ").unwrap();
            }
        }
        write!(buffer, ")").unwrap();
        if let Some(output) = &self.output {
            write!(buffer, " -> ").unwrap();
            output.render_for_error(buffer);
        }
    }
}

impl Display for FQPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let last_segment_index = self.segments.len().saturating_sub(1);
        let mut qself_closing_wedge_index = None;
        if let Some(qself) = &self.qualified_self {
            write!(f, "<{} as ", qself.type_)?;
            qself_closing_wedge_index = Some(qself.position.saturating_sub(1))
        }
        for (i, segment) in self.segments.iter().enumerate() {
            write!(f, "{segment}")?;
            if Some(i) == qself_closing_wedge_index {
                write!(f, ">")?;
            }
            if i != last_segment_index {
                write!(f, "::")?;
            }
        }
        Ok(())
    }
}

impl Display for FQPathType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            FQPathType::ResolvedPath(p) => write!(f, "{p}"),
            FQPathType::Reference(r) => write!(f, "{r}"),
            FQPathType::Tuple(t) => write!(f, "{t}"),
            FQPathType::ScalarPrimitive(s) => {
                write!(f, "{s}")
            }
            FQPathType::Slice(s) => {
                write!(f, "{s}")
            }
            FQPathType::Array(a) => {
                write!(f, "{a}")
            }
            FQPathType::RawPointer(r) => {
                write!(f, "{r}")
            }
            FQPathType::FunctionPointer(fp) => {
                write!(f, "{fp}")
            }
        }
    }
}

impl Display for FQSlice {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}]", self.element)
    }
}

impl Display for FQArray {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}; {}]", self.element, self.len)
    }
}

impl Display for FQResolvedPathType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.path)
    }
}

impl Display for FQReference {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "&")?;
        if self.is_mutable {
            write!(f, "mut ")?;
        }
        write!(f, "{}", self.inner)
    }
}

impl Display for FQRawPointer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.is_mutable {
            write!(f, "*mut ")?;
        } else {
            write!(f, "*const ")?;
        }
        write!(f, "{}", self.inner)
    }
}

impl Display for FQFunctionPointer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write_fn_pointer_prefix(f, &self.abi, self.is_unsafe)?;
        write!(f, "fn(")?;
        let last_input_index = self.inputs.len().saturating_sub(1);
        for (i, input) in self.inputs.iter().enumerate() {
            write!(f, "{input}")?;
            if i != last_input_index && !self.inputs.is_empty() {
                write!(f, ", ")?;
            }
        }
        write!(f, ")")?;
        if let Some(output) = &self.output {
            write!(f, " -> {output}")?;
        }
        Ok(())
    }
}

impl Display for FQTuple {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "(")?;
        let last_element_index = self.elements.len().saturating_sub(1);
        for (i, element) in self.elements.iter().enumerate() {
            write!(f, "{element}")?;
            if i != last_element_index {
                write!(f, ", ")?;
            }
        }
        write!(f, ")")
    }
}

impl Display for FQPathSegment {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.ident)?;
        if !self.generic_arguments.is_empty() {
            write!(f, "::<")?;
        }
        let last_argument_index = self.generic_arguments.len().saturating_sub(1);
        for (j, argument) in self.generic_arguments.iter().enumerate() {
            write!(f, "{argument}")?;
            if j != last_argument_index {
                write!(f, ", ")?;
            }
        }
        if !self.generic_arguments.is_empty() {
            write!(f, ">")?;
        }
        Ok(())
    }
}

impl Display for FQGenericArgument {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            FQGenericArgument::Type(t) => {
                write!(f, "{t}")
            }
            FQGenericArgument::Lifetime(l) => {
                write!(f, "{l}")
            }
        }
    }
}

impl From<ResolvedPathLifetime> for GenericLifetimeParameter {
    fn from(l: ResolvedPathLifetime) -> Self {
        match l {
            ResolvedPathLifetime::Static => GenericLifetimeParameter::Static,
            ResolvedPathLifetime::Named(n) => GenericLifetimeParameter::Named(n),
            ResolvedPathLifetime::Inferred => GenericLifetimeParameter::Inferred,
        }
    }
}

impl From<GenericLifetimeParameter> for ResolvedPathLifetime {
    fn from(l: GenericLifetimeParameter) -> Self {
        match l {
            GenericLifetimeParameter::Static => ResolvedPathLifetime::Static,
            GenericLifetimeParameter::Named(n) => ResolvedPathLifetime::Named(n),
            GenericLifetimeParameter::Inferred => ResolvedPathLifetime::Inferred,
        }
    }
}

impl Display for ResolvedPathLifetime {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ResolvedPathLifetime::Static => write!(f, "'static"),
            ResolvedPathLifetime::Named(name) => write!(f, "'{}", name.as_str()),
            ResolvedPathLifetime::Inferred => write!(f, "'_"),
        }
    }
}
