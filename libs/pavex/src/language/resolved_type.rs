use std::fmt::Write;
use std::fmt::{Debug, Display, Formatter};

use ahash::{HashMap, HashMapExt};
use anyhow::Context;
use bimap::BiHashMap;
use guppy::PackageId;
use serde::{Deserializer, Serializer};

use crate::language::{ImportPath, ResolvedPath, ResolvedPathSegment};

#[derive(serde::Serialize, serde::Deserialize, Eq, PartialEq, Hash, Clone)]
pub enum ResolvedType {
    ResolvedPath(ResolvedPathType),
    Reference(TypeReference),
    Tuple(Tuple),
    ScalarPrimitive(ScalarPrimitive),
    Slice(Slice),
}

impl ResolvedType {
    pub const UNIT_TYPE: ResolvedType = ResolvedType::Tuple(Tuple { elements: vec![] });

    /// Replace unassigned generic type parameters in `templated_type` with the concrete generic type
    /// parameters defined in `bindings`.
    ///
    /// This function can also be used to _partially_ bind the unassigned generic type parameters in
    /// `t`. You are not required to bind all of them.
    pub fn bind_generic_type_parameters(
        &self,
        bindings: &HashMap<NamedTypeGeneric, ResolvedType>,
    ) -> ResolvedType {
        match self {
            ResolvedType::ResolvedPath(t) => {
                let mut bound_generics = Vec::with_capacity(t.generic_arguments.len());
                for generic in &t.generic_arguments {
                    let bound_generic = match generic {
                        GenericArgument::UnassignedTypeParameter(name) => {
                            if let Some(bound_type) = bindings.get(name) {
                                GenericArgument::AssignedTypeParameter(bound_type.clone())
                            } else {
                                generic.to_owned()
                            }
                        }
                        GenericArgument::AssignedTypeParameter(t) => {
                            GenericArgument::AssignedTypeParameter(
                                t.bind_generic_type_parameters(bindings),
                            )
                        }
                        GenericArgument::Lifetime(_) => generic.to_owned(),
                    };
                    bound_generics.push(bound_generic);
                }
                ResolvedType::ResolvedPath(ResolvedPathType {
                    package_id: t.package_id.clone(),
                    // Should we set this to `None`?
                    rustdoc_id: t.rustdoc_id.clone(),
                    base_type: t.base_type.clone(),
                    generic_arguments: bound_generics,
                })
            }
            ResolvedType::Reference(r) => ResolvedType::Reference(TypeReference {
                is_mutable: r.is_mutable,
                inner: Box::new(r.inner.bind_generic_type_parameters(bindings)),
                is_static: r.is_static,
            }),
            ResolvedType::Tuple(t) => {
                let mut bound_elements = Vec::with_capacity(t.elements.len());
                for inner in &t.elements {
                    bound_elements.push(inner.bind_generic_type_parameters(bindings));
                }
                ResolvedType::Tuple(Tuple {
                    elements: bound_elements,
                })
            }
            ResolvedType::ScalarPrimitive(s) => ResolvedType::ScalarPrimitive(s.clone()),
            ResolvedType::Slice(s) => ResolvedType::Slice(Slice {
                element_type: Box::new(s.element_type.bind_generic_type_parameters(bindings)),
            }),
        }
    }

    /// Check if a type can be used as a "template" - i.e. if it has any unassigned generic type parameters.
    #[tracing::instrument(level = "trace", ret)]
    pub fn is_a_template(&self) -> bool {
        match self {
            ResolvedType::ResolvedPath(path) => {
                path.generic_arguments.iter().any(|arg| match arg {
                    GenericArgument::UnassignedTypeParameter(_) => true,
                    GenericArgument::AssignedTypeParameter(g) => g.is_a_template(),
                    GenericArgument::Lifetime(_) => false,
                })
            }
            ResolvedType::Reference(r) => r.inner.is_a_template(),
            ResolvedType::Tuple(t) => t.elements.iter().any(|t| t.is_a_template()),
            ResolvedType::ScalarPrimitive(_) => false,
            ResolvedType::Slice(s) => s.element_type.is_a_template(),
        }
    }

    /// Check if a type can be considered a "template" for another.
    ///
    /// I.e. if by replacing the unassigned generic type parameters of `self` with the
    /// concrete generic type parameters of `concrete_type`, `self` would be equal to `concrete_type`.
    ///
    /// If possible, this function will return a map associating each unassigned generic parameter
    /// in `self` with the type it must be set to in order to match `concrete_type`.
    /// If impossible, this function will return `None`.
    #[tracing::instrument(level = "trace", ret)]
    pub fn is_a_template_for(
        &self,
        concrete_type: &ResolvedType,
    ) -> Option<HashMap<NamedTypeGeneric, ResolvedType>> {
        let mut bindings = HashMap::new();
        if self._is_a_template_for(concrete_type, &mut bindings) {
            Some(bindings)
        } else {
            None
        }
    }

    #[tracing::instrument(level = "trace", ret)]
    fn _is_a_template_for(
        &self,
        concrete_type: &ResolvedType,
        bindings: &mut HashMap<NamedTypeGeneric, ResolvedType>,
    ) -> bool {
        if concrete_type == self {
            return true;
        }
        use ResolvedType::*;
        match (concrete_type, self) {
            (ResolvedPath(concrete_path), ResolvedPath(templated_path)) => {
                templated_path._is_a_resolved_path_type_template_for(concrete_path, bindings)
            }
            (Slice(concrete_slice), Slice(templated_slice)) => templated_slice
                .element_type
                ._is_a_template_for(&concrete_slice.element_type, bindings),
            (Reference(concrete_reference), Reference(templated_reference)) => templated_reference
                .inner
                ._is_a_template_for(&concrete_reference.inner, bindings),
            (Tuple(concrete_tuple), Tuple(templated_tuple)) => {
                if concrete_tuple.elements.len() != templated_tuple.elements.len() {
                    return false;
                }
                concrete_tuple
                    .elements
                    .iter()
                    .zip(templated_tuple.elements.iter())
                    .all(|(concrete_type, templated_type)| {
                        templated_type._is_a_template_for(concrete_type, bindings)
                    })
            }
            (ScalarPrimitive(concrete_primitive), ScalarPrimitive(templated_primitive)) => {
                concrete_primitive == templated_primitive
            }
            (_, _) => false,
        }
    }
}

impl ResolvedPathType {
    fn _is_a_resolved_path_type_template_for(
        &self,
        concrete_type: &ResolvedPathType,
        bindings: &mut HashMap<NamedTypeGeneric, ResolvedType>,
    ) -> bool {
        // We destructure ALL fields to make sure that the compiler reminds us to update
        // this function if we add new fields to `ResolvedPathType`.
        let ResolvedPathType {
            package_id: concrete_package_id,
            rustdoc_id: _,
            base_type: concrete_base_type,
            generic_arguments: concrete_generic_arguments,
        } = concrete_type;
        let ResolvedPathType {
            package_id: templated_package_id,
            rustdoc_id: _,
            base_type: templated_base_type,
            generic_arguments: templated_generic_arguments,
        } = self;
        if concrete_package_id != templated_package_id
            || concrete_base_type != templated_base_type
            || concrete_generic_arguments.len() != templated_generic_arguments.len()
        {
            return false;
        }
        for (concrete_arg, templated_arg) in concrete_generic_arguments
            .iter()
            .zip(templated_generic_arguments.iter())
        {
            use GenericArgument::*;
            match (concrete_arg, templated_arg) {
                (
                    AssignedTypeParameter(concrete_arg_type),
                    AssignedTypeParameter(templated_arg_type),
                ) => {
                    if !templated_arg_type._is_a_template_for(concrete_arg_type, bindings) {
                        return false;
                    }
                }
                (AssignedTypeParameter(assigned), UnassignedTypeParameter(unassigned)) => {
                    // The unassigned type parameter can be assigned to the concrete type
                    // we expect, so it is a specialization.
                    let previous_assignment = bindings.insert(unassigned.clone(), assigned.clone());
                    if let Some(previous_assignment) = previous_assignment {
                        if &previous_assignment != assigned {
                            tracing::trace!(
                            "Type parameter `{:?}` was already assigned to `{:?}` but is now being assigned to `{:?}`",
                            unassigned,
                            previous_assignment,
                            assigned
                        );
                            return false;
                        }
                    }
                }
                (Lifetime(_), Lifetime(_)) => {
                    // Lifetimes are not relevant for specialization (yet).
                }
                (UnassignedTypeParameter(unassigned), _) => {
                    // You are not allowed to specialize a type with an unassigned type parameter.
                    unreachable!("Unassigned type parameter (`{:?}`) in the 'concrete' type (`{:?}`) when checking for specialization", unassigned, concrete_type);
                }
                (AssignedTypeParameter(_), Lifetime(_))
                | (Lifetime(_), UnassignedTypeParameter(_))
                | (Lifetime(_), AssignedTypeParameter(_)) => {
                    return false;
                }
            }
        }
        true
    }
}

#[derive(serde::Serialize, serde::Deserialize, Eq, PartialEq, Hash, Clone)]
pub enum ScalarPrimitive {
    Usize,
    U8,
    U16,
    U32,
    U64,
    Isize,
    I8,
    I16,
    I32,
    I64,
    F32,
    F64,
    Bool,
    Char,
    Str,
}

impl ScalarPrimitive {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Usize => "usize",
            Self::U8 => "u8",
            Self::U16 => "u16",
            Self::U32 => "u32",
            Self::U64 => "u64",
            Self::Isize => "isize",
            Self::I8 => "i8",
            Self::I16 => "i16",
            Self::I32 => "i32",
            Self::I64 => "i64",
            Self::F32 => "f32",
            Self::F64 => "f64",
            Self::Bool => "bool",
            Self::Char => "char",
            Self::Str => "str",
        }
    }
}

impl Debug for ScalarPrimitive {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl Display for ScalarPrimitive {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl TryFrom<&str> for ScalarPrimitive {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let v = match value {
            "usize" => Self::Usize,
            "u8" => Self::U8,
            "u16" => Self::U16,
            "u32" => Self::U32,
            "u64" => Self::U64,
            "isize" => Self::Isize,
            "i8" => Self::I8,
            "i16" => Self::I16,
            "i32" => Self::I32,
            "i64" => Self::I64,
            "f32" => Self::F32,
            "f64" => Self::F64,
            "bool" => Self::Bool,
            "char" => Self::Char,
            "str" => Self::Str,
            _ => anyhow::bail!("Unknown primitive scalar type: {}", value),
        };
        Ok(v)
    }
}

#[derive(serde::Serialize, serde::Deserialize, Eq, PartialEq, Hash, Clone)]
pub struct Tuple {
    pub elements: Vec<ResolvedType>,
}

#[derive(serde::Serialize, serde::Deserialize, Eq, PartialEq, Hash, Clone)]
pub struct Slice {
    pub element_type: Box<ResolvedType>,
}

#[derive(serde::Serialize, serde::Deserialize, Eq, PartialEq, Hash, Clone)]
pub struct TypeReference {
    pub is_mutable: bool,
    pub is_static: bool,
    pub inner: Box<ResolvedType>,
}

#[derive(serde::Serialize, serde::Deserialize, Eq, PartialEq, Hash, Clone)]
pub struct ResolvedPathType {
    #[serde(serialize_with = "serialize_package_id")]
    #[serde(deserialize_with = "deserialize_package_id")]
    // `PackageId` does not implement serde::Deserialize/serde::Serialize, therefore we must
    // manually specify deserializer and serializer to make the whole `ResolvedPathType`
    // (de)serializable.
    pub package_id: PackageId,
    /// The id associated with this type within the (JSON) docs for `package_id`.
    ///
    /// The id is optional to allow for flexible usage patterns - e.g. to leverage [`ResolvedType`]
    /// to work with types that we want to code-generate into a new crate.  
    pub rustdoc_id: Option<rustdoc_types::Id>,
    pub base_type: ImportPath,
    pub generic_arguments: Vec<GenericArgument>,
}

#[derive(serde::Serialize, serde::Deserialize, Eq, PartialEq, Hash, Clone)]
pub enum GenericArgument {
    /// A type parameter that has not been assigned yet, e.g. `T` in `Vec<T>`.
    UnassignedTypeParameter(NamedTypeGeneric),
    /// A type parameter that has been assigned a concrete type, e.g. `u32` in `Vec<u32>`.
    AssignedTypeParameter(ResolvedType),
    /// A lifetime paremeter, e.g. `'a` in `&'a str` or `'static` in `&'static str`.
    Lifetime(Lifetime),
}

#[derive(serde::Serialize, serde::Deserialize, Eq, PartialEq, Hash, Clone, Debug)]
/// A type parameter that has not been assigned yet, e.g. `T` in `Vec<T>`.
pub struct NamedTypeGeneric {
    /// E.g. `T` in `Vec<T>`.
    pub name: String,
}

#[derive(serde::Serialize, serde::Deserialize, Eq, PartialEq, Hash, Clone)]
pub enum Lifetime {
    Static,
}

fn serialize_package_id<S>(package_id: &PackageId, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(package_id.repr())
}

fn deserialize_package_id<'de, D>(deserializer: D) -> Result<PackageId, D::Error>
where
    D: Deserializer<'de>,
{
    let s: &str = serde::de::Deserialize::deserialize(deserializer)?;
    Ok(PackageId::new(s))
}

impl ResolvedType {
    pub fn syn_type(&self, id2name: &BiHashMap<PackageId, String>) -> syn::Type {
        let type_ = self.render_type(id2name);
        syn::parse_str(&type_).unwrap()
    }

    pub fn render_type(&self, id2name: &BiHashMap<PackageId, String>) -> String {
        let mut buffer = String::new();
        self._render_type(id2name, &mut buffer);
        buffer
    }

    fn _render_type(&self, id2name: &BiHashMap<PackageId, String>, buffer: &mut String) {
        match self {
            ResolvedType::ResolvedPath(t) => {
                let crate_name = id2name
                    .get_by_left(&t.package_id)
                    .with_context(|| {
                        format!(
                            "The package id '{}' is missing from the id<>name mapping for crates.",
                            t.package_id
                        )
                    })
                    .unwrap();
                write!(buffer, "{crate_name}").unwrap();
                write!(buffer, "::{}", t.base_type[1..].join("::")).unwrap();
                if !t.generic_arguments.is_empty() {
                    write!(buffer, "<").unwrap();
                    let mut arguments = t.generic_arguments.iter().peekable();
                    while let Some(argument) = arguments.next() {
                        match argument {
                            GenericArgument::AssignedTypeParameter(t) => {
                                write!(buffer, "{}", t.render_type(id2name)).unwrap();
                            }
                            GenericArgument::Lifetime(l) => match l {
                                Lifetime::Static => {
                                    write!(buffer, "'static").unwrap();
                                }
                            },
                            GenericArgument::UnassignedTypeParameter(t) => {
                                write!(buffer, "{}", t.name).unwrap();
                            }
                        }
                        if arguments.peek().is_some() {
                            write!(buffer, ", ").unwrap();
                        }
                    }
                    write!(buffer, ">").unwrap();
                }
            }
            ResolvedType::Reference(r) => {
                write!(buffer, "&").unwrap();
                if r.is_static {
                    write!(buffer, "'static ").unwrap();
                }
                if r.is_mutable {
                    write!(buffer, "mut ").unwrap();
                }
                r.inner._render_type(id2name, buffer);
            }
            ResolvedType::Tuple(t) => {
                write!(buffer, "(").unwrap();
                let mut elements = t.elements.iter().peekable();
                while let Some(element) = elements.next() {
                    element._render_type(id2name, buffer);
                    if elements.peek().is_some() {
                        write!(buffer, ", ").unwrap();
                    }
                }
                write!(buffer, ")").unwrap();
            }
            ResolvedType::ScalarPrimitive(s) => {
                write!(buffer, "{s}").unwrap();
            }
            ResolvedType::Slice(s) => {
                write!(buffer, "[{}]", s.element_type.render_type(id2name)).unwrap();
            }
        }
    }
}

impl ResolvedPathType {
    pub fn resolved_path(&self) -> ResolvedPath {
        let mut segments = Vec::with_capacity(self.base_type.len());
        for segment in &self.base_type {
            segments.push(ResolvedPathSegment {
                ident: segment.to_owned(),
                generic_arguments: vec![],
            });
        }
        ResolvedPath {
            segments,
            qualified_self: None,
            package_id: self.package_id.clone(),
        }
    }
}

impl std::fmt::Debug for ResolvedType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ResolvedType::ResolvedPath(t) => write!(f, "{t:?}"),
            ResolvedType::Reference(r) => write!(f, "{r:?}"),
            ResolvedType::Tuple(t) => write!(f, "{t:?}"),
            ResolvedType::ScalarPrimitive(s) => write!(f, "{s:?}"),
            ResolvedType::Slice(s) => write!(f, "{s:?}"),
        }
    }
}

impl std::fmt::Debug for GenericArgument {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            GenericArgument::AssignedTypeParameter(r) => write!(f, "{r:?}"),
            GenericArgument::Lifetime(l) => write!(f, "{l:?}"),
            GenericArgument::UnassignedTypeParameter(t) => write!(f, "{}", t.name),
        }
    }
}

impl std::fmt::Debug for Lifetime {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Lifetime::Static => write!(f, "'static"),
        }
    }
}

impl std::fmt::Debug for ResolvedPathType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.base_type.join("::"))?;
        if !self.generic_arguments.is_empty() {
            write!(f, "<")?;
            let mut arguments = self.generic_arguments.iter().peekable();
            while let Some(argument) = arguments.next() {
                write!(f, "{argument:?}")?;
                if arguments.peek().is_some() {
                    write!(f, ", ")?;
                }
            }
            write!(f, ">")?;
        }
        Ok(())
    }
}

impl std::fmt::Debug for Slice {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{:?}]", self.element_type)
    }
}

impl std::fmt::Debug for Tuple {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "(")?;
        let mut elements = self.elements.iter().peekable();
        while let Some(element) = elements.next() {
            write!(f, "{element:?}")?;
            if elements.peek().is_some() {
                write!(f, ", ")?;
            }
        }
        write!(f, ")")?;
        Ok(())
    }
}

impl std::fmt::Debug for TypeReference {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "&")?;
        if self.is_static {
            write!(f, "'static ")?;
        }
        if self.is_mutable {
            write!(f, "mut ")?;
        }
        write!(f, "{:?}", self.inner)?;
        Ok(())
    }
}

impl From<Tuple> for ResolvedType {
    fn from(value: Tuple) -> Self {
        Self::Tuple(value)
    }
}

impl From<ResolvedPathType> for ResolvedType {
    fn from(value: ResolvedPathType) -> Self {
        Self::ResolvedPath(value)
    }
}

impl From<TypeReference> for ResolvedType {
    fn from(value: TypeReference) -> Self {
        Self::Reference(value)
    }
}
