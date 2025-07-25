use std::fmt::Write;
use std::fmt::{Debug, Display, Formatter};
use std::hash::{Hash, Hasher};

use ahash::{HashMap, HashMapExt};
use anyhow::Context;
use bimap::BiHashMap;
use guppy::PackageId;
use indexmap::{IndexMap, IndexSet};
use serde::{Deserializer, Serializer};

use crate::language::resolved_type::generics_equivalence::UnassignedIdGenerator;
use crate::language::{FQPath, FQPathSegment};

#[derive(serde::Serialize, serde::Deserialize, Eq, PartialEq, Hash, Clone)]
pub enum ResolvedType {
    ResolvedPath(PathType),
    Reference(TypeReference),
    Tuple(Tuple),
    ScalarPrimitive(ScalarPrimitive),
    Slice(Slice),
    Generic(Generic),
}

impl AsRef<ResolvedType> for ResolvedType {
    fn as_ref(&self) -> &ResolvedType {
        self
    }
}

impl ResolvedType {
    pub const UNIT_TYPE: ResolvedType = ResolvedType::Tuple(Tuple { elements: vec![] });

    /// Returns `true` if `t` is a `Result` type.
    pub fn is_result(&self) -> bool {
        let ResolvedType::ResolvedPath(t) = self else {
            return false;
        };
        t.base_type == ["core", "result", "Result"]
            || t.base_type == ["core", "prelude", "rust_2015", "Result"]
            || t.base_type == ["core", "prelude", "rust_2018", "Result"]
            || t.base_type == ["core", "prelude", "rust_2021", "Result"]
    }

    /// Replace unassigned generic type parameters in `templated_type` with the concrete generic type
    /// parameters defined in `bindings`.
    ///
    /// This function can also be used to _partially_ bind the unassigned generic type parameters in
    /// `t`. You are not required to bind all of them.
    pub fn bind_generic_type_parameters(
        &self,
        bindings: &HashMap<String, ResolvedType>,
    ) -> ResolvedType {
        match self {
            ResolvedType::ResolvedPath(t) => {
                let mut bound_generics = Vec::with_capacity(t.generic_arguments.len());
                for generic in &t.generic_arguments {
                    let bound_generic = match generic {
                        GenericArgument::TypeParameter(t) => {
                            GenericArgument::TypeParameter(t.bind_generic_type_parameters(bindings))
                        }
                        GenericArgument::Lifetime(_) => generic.to_owned(),
                    };
                    bound_generics.push(bound_generic);
                }
                ResolvedType::ResolvedPath(PathType {
                    package_id: t.package_id.clone(),
                    // Should we set this to `None`?
                    rustdoc_id: t.rustdoc_id,
                    base_type: t.base_type.clone(),
                    generic_arguments: bound_generics,
                })
            }
            ResolvedType::Reference(r) => ResolvedType::Reference(TypeReference {
                is_mutable: r.is_mutable,
                inner: Box::new(r.inner.bind_generic_type_parameters(bindings)),
                lifetime: r.lifetime.clone(),
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
            ResolvedType::Generic(g) => {
                if let Some(bound_type) = bindings.get(&g.name) {
                    bound_type.clone()
                } else {
                    ResolvedType::Generic(g.to_owned())
                }
            }
        }
    }

    /// Check if a type can be used as a "template"—i.e. if it has any unassigned generic parameters.
    #[tracing::instrument(level = "trace", ret)]
    pub fn is_a_template(&self) -> bool {
        match self {
            ResolvedType::ResolvedPath(path) => {
                path.generic_arguments.iter().any(|arg| match arg {
                    GenericArgument::TypeParameter(g) => g.is_a_template(),
                    GenericArgument::Lifetime(GenericLifetimeParameter::Static) => false,
                    // One might want to do a more precise level of analysis wrt lifetimes,
                    // but for now we just assume that named lifetimes are not relevant for
                    // specialization.
                    GenericArgument::Lifetime(GenericLifetimeParameter::Named(_)) => false,
                })
            }
            ResolvedType::Reference(r) => r.inner.is_a_template(),
            ResolvedType::Tuple(t) => t.elements.iter().any(|t| t.is_a_template()),
            ResolvedType::ScalarPrimitive(_) => false,
            ResolvedType::Slice(s) => s.element_type.is_a_template(),
            ResolvedType::Generic(_) => true,
        }
    }

    /// Returns the set of all unassigned generic type parameters in this type.
    ///
    /// E.g. `[T]` for `Json<T, u8>` or `[T, V]` for `Json<T, V>`.
    pub fn unassigned_generic_type_parameters(&self) -> IndexSet<String> {
        let mut set = IndexSet::new();
        self._unassigned_generic_type_parameters(&mut set);
        set
    }

    fn _unassigned_generic_type_parameters(&self, set: &mut IndexSet<String>) {
        match self {
            ResolvedType::ResolvedPath(path) => {
                for arg in &path.generic_arguments {
                    match arg {
                        GenericArgument::TypeParameter(g) => {
                            g._unassigned_generic_type_parameters(set);
                        }
                        GenericArgument::Lifetime(_) => {}
                    }
                }
            }
            ResolvedType::Reference(r) => r.inner._unassigned_generic_type_parameters(set),
            ResolvedType::Tuple(t) => {
                for inner in &t.elements {
                    inner._unassigned_generic_type_parameters(set);
                }
            }
            ResolvedType::ScalarPrimitive(_) => {}
            ResolvedType::Slice(s) => s.element_type._unassigned_generic_type_parameters(set),
            ResolvedType::Generic(t) => {
                set.insert(t.name.clone());
            }
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
    ) -> Option<HashMap<String, ResolvedType>> {
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
        bindings: &mut HashMap<String, ResolvedType>,
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
            (_, Generic(parameter)) => {
                bindings.insert(parameter.name.clone(), concrete_type.clone());
                true
            }
            (_, _) => false,
        }
    }

    /// Check if, by renaming the unassigned generic type parameters of `self` (via a bijection!),
    /// `self` would be equal to `other`.
    /// If possible, this function will return a map associating each unassigned generic parameter
    /// in `self` with the name it must be renamed to in order to match `other`.
    /// If impossible, this function will return `None`.
    pub(crate) fn is_equivalent_to<'a, 'b>(
        &'a self,
        other: &'b ResolvedType,
    ) -> Option<HashMap<&'a str, &'b str>> {
        let mut self_id_gen = UnassignedIdGenerator::new();
        let mut other_id_gen = UnassignedIdGenerator::new();
        if self._is_equivalent_to(other, &mut self_id_gen, &mut other_id_gen) {
            Some(
                self_id_gen
                    .into_iter()
                    .zip(other_id_gen.into_iter())
                    .map(|((self_name, _), (other_name, _))| (self_name, other_name))
                    .collect(),
            )
        } else {
            None
        }
    }

    fn _is_equivalent_to<'a, 'b>(
        &'a self,
        other: &'b ResolvedType,
        self_id_gen: &mut UnassignedIdGenerator<'a>,
        other_id_gen: &mut UnassignedIdGenerator<'b>,
    ) -> bool {
        use ResolvedType::*;
        match (self, other) {
            (ResolvedPath(self_path), ResolvedPath(other_path)) => {
                self_path._is_equivalent_to(other_path, self_id_gen, other_id_gen)
            }
            (Slice(self_slice), Slice(other_slice)) => self_slice.element_type._is_equivalent_to(
                &other_slice.element_type,
                self_id_gen,
                other_id_gen,
            ),
            (Reference(self_reference), Reference(other_reference)) => self_reference
                .inner
                ._is_equivalent_to(&other_reference.inner, self_id_gen, other_id_gen),
            (Tuple(self_tuple), Tuple(other_tuple)) => self_tuple
                .elements
                .iter()
                .zip(other_tuple.elements.iter())
                .all(|(self_type, other_type)| {
                    self_type._is_equivalent_to(other_type, self_id_gen, other_id_gen)
                }),
            (ScalarPrimitive(self_p), ScalarPrimitive(other_p)) => self_p == other_p,
            (Generic(self_g), Generic(other_g)) => {
                let first_id = self_id_gen.id(&self_g.name);
                let second_id = other_id_gen.id(&other_g.name);
                first_id == second_id
            }
            (_, _) => false,
        }
    }

    /// Return `true` if there is at least one elided lifetime parameter in this type.
    ///
    /// E.g. `&'_ str` and `&str` would both return `true`. `&'static str` or `&'a str` wouldn't.
    pub fn has_implicit_lifetime_parameters(&self) -> bool {
        match self {
            ResolvedType::ResolvedPath(path) => {
                path.generic_arguments.iter().any(|arg| match arg {
                    GenericArgument::TypeParameter(g) => g.has_implicit_lifetime_parameters(),
                    GenericArgument::Lifetime(GenericLifetimeParameter::Named(l)) if l == "_" => {
                        true
                    }
                    GenericArgument::Lifetime(GenericLifetimeParameter::Named(_))
                    | GenericArgument::Lifetime(GenericLifetimeParameter::Static) => false,
                })
            }
            ResolvedType::Reference(r) => {
                match &r.lifetime {
                    Lifetime::Named(s) if s == "_" => {
                        return true;
                    }
                    Lifetime::Elided => {
                        return true;
                    }
                    Lifetime::Named(_) | Lifetime::Static => {}
                }
                r.inner.has_implicit_lifetime_parameters()
            }
            ResolvedType::Tuple(t) => t
                .elements
                .iter()
                .any(|t| t.has_implicit_lifetime_parameters()),
            ResolvedType::ScalarPrimitive(_) => false,
            ResolvedType::Slice(s) => s.element_type.has_implicit_lifetime_parameters(),
            ResolvedType::Generic(_) => false,
        }
    }

    /// Replace all implicit lifetimes (e.g. `&'_ str` or the elided lifetime in `&str`) to
    /// the provided named lifetime.
    pub fn set_implicit_lifetimes(&mut self, inferred_lifetime: String) {
        match self {
            ResolvedType::ResolvedPath(path) => {
                for arg in path.generic_arguments.iter_mut() {
                    if let GenericArgument::Lifetime(lifetime) = arg {
                        if let GenericLifetimeParameter::Named(name) = lifetime {
                            if name == "_" {
                                *lifetime =
                                    GenericLifetimeParameter::Named(inferred_lifetime.clone());
                            }
                        }
                    }
                }
            }
            ResolvedType::Reference(r) => {
                match &mut r.lifetime {
                    Lifetime::Named(s) if s == "_" => {
                        r.lifetime = Lifetime::Named(inferred_lifetime.clone());
                    }
                    Lifetime::Elided => {
                        r.lifetime = Lifetime::Named(inferred_lifetime.clone());
                    }
                    Lifetime::Static | Lifetime::Named(_) => {}
                }
                r.inner.set_implicit_lifetimes(inferred_lifetime);
            }
            ResolvedType::Tuple(t) => t
                .elements
                .iter_mut()
                .for_each(|e| e.set_implicit_lifetimes(inferred_lifetime.clone())),
            ResolvedType::Slice(s) => s.element_type.set_implicit_lifetimes(inferred_lifetime),
            ResolvedType::Generic(_) | ResolvedType::ScalarPrimitive(_) => {}
        }
    }

    /// Rename named lifetime parameters in this type according to the provided mapping.
    ///
    /// You don't need to provide a mapping for lifetimes that you don't want to rename.
    pub fn rename_lifetime_parameters(&mut self, original2renamed: &IndexMap<String, String>) {
        match self {
            ResolvedType::ResolvedPath(t) => {
                for arg in t.generic_arguments.iter_mut() {
                    match arg {
                        GenericArgument::TypeParameter(tp) => {
                            tp.rename_lifetime_parameters(original2renamed);
                        }
                        GenericArgument::Lifetime(l) => {
                            if let GenericLifetimeParameter::Named(l) = l {
                                if let Some(new_name) = original2renamed.get(l) {
                                    *l = new_name.clone();
                                }
                            }
                        }
                    }
                }
            }
            ResolvedType::Reference(r) => {
                match &mut r.lifetime {
                    Lifetime::Named(l) => {
                        if let Some(new_name) = original2renamed.get(l) {
                            *l = new_name.clone();
                        }
                    }
                    Lifetime::Static | Lifetime::Elided => {}
                }
                r.inner.rename_lifetime_parameters(original2renamed);
            }
            ResolvedType::Tuple(t) => {
                for e in t.elements.iter_mut() {
                    e.rename_lifetime_parameters(original2renamed);
                }
            }
            ResolvedType::Slice(s) => {
                s.element_type.rename_lifetime_parameters(original2renamed);
            }
            ResolvedType::Generic(_) | ResolvedType::ScalarPrimitive(_) => {}
        }
    }

    /// Return the set of all lifetime parameters for this type.
    pub fn lifetime_parameters(&self) -> IndexSet<Lifetime> {
        let mut set = IndexSet::new();
        self._lifetime_parameters(&mut set);
        set
    }

    fn _lifetime_parameters(&self, set: &mut IndexSet<Lifetime>) {
        match self {
            ResolvedType::ResolvedPath(path) => {
                for arg in &path.generic_arguments {
                    match arg {
                        GenericArgument::TypeParameter(g) => {
                            g._lifetime_parameters(set);
                        }
                        GenericArgument::Lifetime(GenericLifetimeParameter::Static) => {
                            set.insert(Lifetime::Static);
                        }
                        GenericArgument::Lifetime(GenericLifetimeParameter::Named(l)) => {
                            if l != "_" {
                                set.insert(Lifetime::Named(l.into()));
                            } else {
                                set.insert(Lifetime::Elided);
                            }
                        }
                    }
                }
            }
            ResolvedType::Reference(r) => {
                set.insert(r.lifetime.clone());
                r.inner._lifetime_parameters(set)
            }
            ResolvedType::Tuple(t) => {
                for inner in &t.elements {
                    inner._lifetime_parameters(set);
                }
            }
            ResolvedType::Slice(s) => s.element_type._lifetime_parameters(set),
            ResolvedType::ScalarPrimitive(_) | ResolvedType::Generic(_) => {}
        }
    }

    /// Return the set of free lifetime parameters (i.e. non `'static`) for this type.
    pub fn named_lifetime_parameters(&self) -> IndexSet<String> {
        let mut set = IndexSet::new();
        self._named_lifetime_parameters(&mut set);
        set
    }

    fn _named_lifetime_parameters(&self, set: &mut IndexSet<String>) {
        match self {
            ResolvedType::ResolvedPath(path) => {
                for arg in &path.generic_arguments {
                    match arg {
                        GenericArgument::TypeParameter(g) => {
                            g._named_lifetime_parameters(set);
                        }
                        GenericArgument::Lifetime(GenericLifetimeParameter::Static) => {}
                        GenericArgument::Lifetime(GenericLifetimeParameter::Named(l)) => {
                            if l != "_" {
                                set.insert(l.clone());
                            }
                        }
                    }
                }
            }
            ResolvedType::Reference(r) => {
                match &r.lifetime {
                    Lifetime::Named(l) => {
                        if l != "_" {
                            set.insert(l.clone());
                        }
                    }
                    Lifetime::Static | Lifetime::Elided => {}
                }
                r.inner._named_lifetime_parameters(set)
            }
            ResolvedType::Tuple(t) => {
                for inner in &t.elements {
                    inner._named_lifetime_parameters(set);
                }
            }
            ResolvedType::Slice(s) => s.element_type._named_lifetime_parameters(set),
            ResolvedType::ScalarPrimitive(_) | ResolvedType::Generic(_) => {}
        }
    }

    pub(crate) fn display_for_error(&self) -> String {
        let mut s = String::new();
        self._display_for_error(&mut s);
        s
    }

    fn _display_for_error<W: std::fmt::Write>(&self, buffer: &mut W) {
        match self {
            ResolvedType::ResolvedPath(t) => {
                write!(buffer, "{}", t.base_type.join("::")).unwrap();
                if !t.generic_arguments.is_empty() {
                    write!(buffer, "<").unwrap();
                    let mut arguments = t.generic_arguments.iter().peekable();
                    while let Some(argument) = arguments.next() {
                        match argument {
                            GenericArgument::TypeParameter(t) => {
                                t._display_for_error(buffer);
                            }
                            GenericArgument::Lifetime(l) => match l {
                                GenericLifetimeParameter::Static => {
                                    write!(buffer, "'static").unwrap();
                                }
                                GenericLifetimeParameter::Named(l) => {
                                    write!(buffer, "'{l}").unwrap();
                                }
                            },
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
                match &r.lifetime {
                    Lifetime::Static => {
                        write!(buffer, "'static ").unwrap();
                    }
                    Lifetime::Named(l) => {
                        write!(buffer, "'{l} ").unwrap();
                    }
                    Lifetime::Elided => {}
                }
                if r.is_mutable {
                    write!(buffer, "mut ").unwrap();
                }
                r.inner._display_for_error(buffer);
            }
            ResolvedType::Tuple(t) => {
                write!(buffer, "(").unwrap();
                let mut elements = t.elements.iter().peekable();
                while let Some(element) = elements.next() {
                    element._display_for_error(buffer);
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
                write!(buffer, "[").unwrap();
                s.element_type._display_for_error(buffer);
                write!(buffer, "]").unwrap();
            }
            ResolvedType::Generic(t) => {
                write!(buffer, "{}", t.name).unwrap();
            }
        }
    }
}

impl PathType {
    fn _is_a_resolved_path_type_template_for(
        &self,
        concrete_type: &PathType,
        bindings: &mut HashMap<String, ResolvedType>,
    ) -> bool {
        // We destructure ALL fields to make sure that the compiler reminds us to update
        // this function if we add new fields to `ResolvedPathType`.
        let PathType {
            package_id: concrete_package_id,
            rustdoc_id: _,
            base_type: concrete_base_type,
            generic_arguments: concrete_generic_arguments,
        } = concrete_type;
        let PathType {
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
                (TypeParameter(ResolvedType::Generic(unassigned)), _) => {
                    // You are not allowed to specialize a type with an unassigned type parameter.
                    unreachable!(
                        "Unassigned type parameter (`{:?}`) in the 'concrete' type (`{:?}`) when checking for specialization",
                        unassigned, concrete_type
                    );
                }
                (TypeParameter(assigned), TypeParameter(ResolvedType::Generic(unassigned))) => {
                    // The unassigned type parameter can be assigned to the concrete type
                    // we expect, so it is a specialization.
                    let previous_assignment =
                        bindings.insert(unassigned.name.clone(), assigned.clone());
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
                (TypeParameter(concrete_arg_type), TypeParameter(templated_arg_type)) => {
                    if !templated_arg_type._is_a_template_for(concrete_arg_type, bindings) {
                        return false;
                    }
                }
                (Lifetime(_), Lifetime(_)) => {
                    // Lifetimes are not relevant for specialization (yet).
                }
                (TypeParameter(_), Lifetime(_)) | (Lifetime(_), TypeParameter(_)) => {
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
    U128,
    Isize,
    I8,
    I16,
    I32,
    I64,
    I128,
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
            Self::U128 => "u128",
            Self::Isize => "isize",
            Self::I8 => "i8",
            Self::I16 => "i16",
            Self::I32 => "i32",
            Self::I64 => "i64",
            Self::I128 => "i128",
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

#[derive(thiserror::Error, Debug)]
#[error("Unknown primitive type, `{name}`")]
pub struct UnknownPrimitive {
    pub name: String,
}

impl TryFrom<&str> for ScalarPrimitive {
    type Error = UnknownPrimitive;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let v = match value {
            "usize" => Self::Usize,
            "u8" => Self::U8,
            "u16" => Self::U16,
            "u32" => Self::U32,
            "u64" => Self::U64,
            "u128" => Self::U128,
            "isize" => Self::Isize,
            "i8" => Self::I8,
            "i16" => Self::I16,
            "i32" => Self::I32,
            "i64" => Self::I64,
            "i128" => Self::I128,
            "f32" => Self::F32,
            "f64" => Self::F64,
            "bool" => Self::Bool,
            "char" => Self::Char,
            "str" => Self::Str,
            _ => {
                return Err(UnknownPrimitive {
                    name: value.to_string(),
                });
            }
        };
        Ok(v)
    }
}

#[derive(serde::Serialize, serde::Deserialize, Eq, PartialEq, Hash, Clone)]
/// An unassigned generic parameter—e.g. `T` in `fn foo<T>(t: T)`.
pub struct Generic {
    pub name: String,
}

#[derive(serde::Serialize, serde::Deserialize, Eq, PartialEq, Hash, Clone)]
/// A Rust tuple—e.g. `(u8, u16, u32)`.
pub struct Tuple {
    pub elements: Vec<ResolvedType>,
}

#[derive(serde::Serialize, serde::Deserialize, Eq, PartialEq, Hash, Clone)]
/// A Rust slice—e.g. `[u16]`.
pub struct Slice {
    pub element_type: Box<ResolvedType>,
}

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash, Clone)]
/// A Rust reference—e.g. `&mut u32` or `&'static mut Vec<u8>`.
pub struct TypeReference {
    pub is_mutable: bool,
    pub lifetime: Lifetime,
    pub inner: Box<ResolvedType>,
}

#[derive(serde::Serialize, serde::Deserialize, Eq, Clone)]
pub struct PathType {
    #[serde(serialize_with = "serialize_package_id")]
    #[serde(deserialize_with = "deserialize_package_id")]
    // `PackageId` doesn't implement serde::Deserialize/serde::Serialize, therefore we must
    // manually specify deserializer and serializer to make the whole `ResolvedPathType`
    // (de)serializable.
    pub package_id: PackageId,
    /// The id associated with this type within the (JSON) docs for `package_id`.
    ///
    /// The id is optional to allow for flexible usage patterns—e.g. to leverage [`ResolvedType`]
    /// to work with types that we want to code-generate into a new crate.
    pub rustdoc_id: Option<rustdoc_types::Id>,
    pub base_type: Vec<String>,
    pub generic_arguments: Vec<GenericArgument>,
}

impl PathType {
    pub(crate) fn _is_equivalent_to<'a, 'b>(
        &'a self,
        other: &'b PathType,
        self_id_gen: &mut UnassignedIdGenerator<'a>,
        other_id_gen: &mut UnassignedIdGenerator<'b>,
    ) -> bool {
        if (self.package_id != other.package_id)
            || (self.rustdoc_id != other.rustdoc_id)
            || (self.base_type != other.base_type)
        {
            return false;
        }
        let self_args = &self.generic_arguments;
        let other_args = &other.generic_arguments;
        if self_args.len() != other_args.len() {
            return false;
        }
        for (self_arg, other_arg) in self_args.iter().zip(other_args) {
            use GenericArgument::*;
            use ResolvedType::*;
            match (self_arg, other_arg) {
                (TypeParameter(Generic(first)), TypeParameter(Generic(second))) => {
                    let first_id = self_id_gen.id(&first.name);
                    let second_id = other_id_gen.id(&second.name);
                    if first_id != second_id {
                        return false;
                    }
                }
                (TypeParameter(first), TypeParameter(second)) => {
                    if !first._is_equivalent_to(second, self_id_gen, other_id_gen) {
                        return false;
                    }
                }
                (Lifetime(_), Lifetime(_)) => {
                    // Lifetimes are not relevant for specialization (yet).
                }
                (first, second) => {
                    if first != second {
                        return false;
                    }
                }
            }
        }
        true
    }
}

mod generics_equivalence {
    use ahash::{HashMap, HashMapExt};

    /// To make the comparison easier, we assign a monotonically increasing unique id to all
    /// unassigned generic parameters.
    /// If the ids match, we know that the two sequences of unassigned generic parameters are equivalent.
    pub(crate) struct UnassignedIdGenerator<'a> {
        next_id: usize,
        known_ids: HashMap<&'a str, usize>,
    }

    impl<'a> UnassignedIdGenerator<'a> {
        pub(super) fn new() -> Self {
            Self {
                next_id: 0,
                known_ids: HashMap::new(),
            }
        }

        pub(super) fn id<'b>(&'b mut self, name: &'a str) -> usize
        where
            'a: 'b,
        {
            if let Some(id) = self.known_ids.get(&name) {
                *id
            } else {
                let id = self.next_id;
                self.next_id += 1;
                self.known_ids.insert(name, id);
                id
            }
        }

        pub(super) fn into_iter(self) -> impl Iterator<Item = (&'a str, usize)> {
            self.known_ids.into_iter()
        }
    }
}

impl PartialEq for PathType {
    fn eq(&self, other: &Self) -> bool {
        self._is_equivalent_to(
            other,
            &mut UnassignedIdGenerator::new(),
            &mut UnassignedIdGenerator::new(),
        )
    }
}

impl Hash for PathType {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let Self {
            package_id,
            rustdoc_id,
            base_type,
            generic_arguments,
        } = self;
        package_id.hash(state);
        rustdoc_id.hash(state);
        base_type.hash(state);
        let mut id_gen = UnassignedIdGenerator::new();
        for generic_argument in generic_arguments {
            match generic_argument {
                GenericArgument::Lifetime(lifetime) => {
                    state.write_u8(0);
                    lifetime.hash(state);
                }
                GenericArgument::TypeParameter(ResolvedType::Generic(
                    unassigned_type_parameter,
                )) => {
                    state.write_u8(1);
                    id_gen.id(&unassigned_type_parameter.name).hash(state);
                }
                _ => {
                    state.write_u8(1);
                    generic_argument.hash(state);
                }
            }
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Eq, PartialEq, Hash, Clone)]
pub enum GenericArgument {
    /// A generic type parameter, e.g. `u32` in `Vec<u32>` or `T` in `HashSet<T>`.
    TypeParameter(ResolvedType),
    /// A lifetime parameter, e.g. `'a` in `Cow<'a, str>`.
    Lifetime(GenericLifetimeParameter),
}

#[derive(serde::Serialize, serde::Deserialize, Eq, Clone)]
pub enum GenericLifetimeParameter {
    Named(String),
    Static,
}

impl Display for GenericLifetimeParameter {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            GenericLifetimeParameter::Named(s) => write!(f, "'{s}"),
            GenericLifetimeParameter::Static => write!(f, "'static"),
        }
    }
}

impl PartialEq for GenericLifetimeParameter {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (GenericLifetimeParameter::Static, GenericLifetimeParameter::Static) => true,
            (GenericLifetimeParameter::Static, _) => false,
            // We don't care about the name of the lifetime, only that it is not static.
            _ => true,
        }
    }
}

impl Hash for GenericLifetimeParameter {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            GenericLifetimeParameter::Static => {
                state.write_u8(0);
            }
            GenericLifetimeParameter::Named(_) => {
                // We don't care about the name of the lifetime, only that it is not static.
                state.write_u8(1);
            }
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Eq, Clone)]
pub enum Lifetime {
    /// The `'static` lifetime.
    Static,
    /// A named lifetime, e.g. `'a` in `&'a str`.
    /// It also include the "inferred" lifetime, which is represented as `'_`.
    Named(String),
    /// A lifetime that is omitted from the source thanks to lifetime elision
    /// (see https://doc.rust-lang.org/nomicon/lifetime-elision.html).
    ///
    /// E.g. `&str`.
    Elided,
}

impl PartialEq for Lifetime {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Lifetime::Static, Lifetime::Static) => true,
            (Lifetime::Static, _) => false,
            (_, Lifetime::Static) => false,
            // We don't care about the name of the lifetime, only that it is not static.
            _ => true,
        }
    }
}

impl Hash for Lifetime {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Lifetime::Static => {
                state.write_u8(0);
            }
            Lifetime::Named(_) | Lifetime::Elided => {
                // We don't care about the name of the lifetime, only that it is not static.
                state.write_u8(1);
            }
        }
    }
}

impl From<Option<String>> for Lifetime {
    fn from(s: Option<String>) -> Self {
        match s {
            Some(s) => {
                if &s == "'static" {
                    Lifetime::Static
                } else {
                    Lifetime::Named(s)
                }
            }
            None => Lifetime::Elided,
        }
    }
}

impl Lifetime {
    pub fn is_static(&self) -> bool {
        match self {
            Lifetime::Named(_) | Lifetime::Elided => false,
            Lifetime::Static => true,
        }
    }

    pub fn is_elided(&self) -> bool {
        match self {
            Lifetime::Named(n) if n == "_" => true,
            Lifetime::Elided => true,
            _ => false,
        }
    }
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
                            GenericArgument::TypeParameter(t) => {
                                write!(buffer, "{}", t.render_type(id2name)).unwrap();
                            }
                            GenericArgument::Lifetime(l) => match l {
                                GenericLifetimeParameter::Static => {
                                    write!(buffer, "'static").unwrap();
                                }
                                GenericLifetimeParameter::Named(l) => {
                                    write!(buffer, "'{l}").unwrap();
                                }
                            },
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
                match &r.lifetime {
                    Lifetime::Static => {
                        write!(buffer, "'static ").unwrap();
                    }
                    Lifetime::Named(l) => {
                        write!(buffer, "'{l} ").unwrap();
                    }
                    Lifetime::Elided => {}
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
            ResolvedType::Generic(t) => {
                write!(buffer, "{}", t.name).unwrap();
            }
        }
    }
}

impl PathType {
    pub fn resolved_path(&self) -> FQPath {
        let mut segments = Vec::with_capacity(self.base_type.len());
        for segment in &self.base_type {
            segments.push(FQPathSegment {
                ident: segment.to_owned(),
                generic_arguments: vec![],
            });
        }
        FQPath {
            segments,
            qualified_self: None,
            package_id: self.package_id.clone(),
        }
    }
}

impl Debug for ResolvedType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ResolvedType::ResolvedPath(t) => write!(f, "{t:?}"),
            ResolvedType::Reference(r) => write!(f, "{r:?}"),
            ResolvedType::Tuple(t) => write!(f, "{t:?}"),
            ResolvedType::ScalarPrimitive(s) => write!(f, "{s:?}"),
            ResolvedType::Slice(s) => write!(f, "{s:?}"),
            ResolvedType::Generic(g) => write!(f, "{g:?}"),
        }
    }
}

impl Debug for Generic {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl Debug for GenericArgument {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            GenericArgument::TypeParameter(r) => write!(f, "{r:?}"),
            GenericArgument::Lifetime(l) => write!(f, "{l:?}"),
        }
    }
}

impl Debug for Lifetime {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Lifetime::Static => write!(f, "'static"),
            Lifetime::Named(name) => write!(f, "'{name}"),
            Lifetime::Elided => Ok(()),
        }
    }
}

impl Debug for GenericLifetimeParameter {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            GenericLifetimeParameter::Named(name) => write!(f, "'{name}"),
            GenericLifetimeParameter::Static => write!(f, "'static"),
        }
    }
}

impl Debug for PathType {
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

impl Debug for Slice {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{:?}]", self.element_type)
    }
}

impl Debug for Tuple {
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

impl Debug for TypeReference {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "&")?;
        if self.lifetime != Lifetime::Elided {
            write!(f, "{:?} ", self.lifetime)?;
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

impl From<PathType> for ResolvedType {
    fn from(value: PathType) -> Self {
        Self::ResolvedPath(value)
    }
}

impl From<TypeReference> for ResolvedType {
    fn from(value: TypeReference) -> Self {
        Self::Reference(value)
    }
}

#[cfg(test)]
mod tests {
    use ahash::{HashSet, HashSetExt};

    use crate::language::{GenericLifetimeParameter, Lifetime};

    #[test]
    fn all_named_lifetimes_are_equivalent() {
        let lifetimes = vec![
            Lifetime::Named("a".to_string()),
            Lifetime::Named("b".to_string()),
            Lifetime::Elided,
        ];
        for first in &lifetimes {
            for second in &lifetimes {
                assert_eq!(first, second);
            }
        }

        let mut set = HashSet::new();
        set.insert(Lifetime::Named("a".into()));
        for lifetime in &lifetimes {
            assert!(set.contains(lifetime));
        }
    }

    #[test]
    fn all_named_generic_lifetimes_are_equivalent() {
        let named1 = GenericLifetimeParameter::Named("a".to_string());
        let named2 = GenericLifetimeParameter::Named("b".to_string());

        assert_eq!(named1, named2);

        let mut set = HashSet::new();
        set.insert(named1);
        assert!(set.contains(&named2));
    }
}
