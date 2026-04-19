use std::fmt::{Debug, Formatter};

use ahash::{HashMap, HashMapExt};
use indexmap::{IndexMap, IndexSet};
use rustdoc_ext::GlobalItemId;

use crate::generics_equivalence::UnassignedIdGenerator;
use crate::{
    Array, CanonicalPathResolver, FunctionPointer, FunctionPointerInput, Generic, GenericArgument,
    GenericLifetimeParameter, Lifetime, NamedLifetime, PathType, RawPointer, Slice, Tuple, Type,
    TypeReference,
};

/// A `Type` with canonicalized names for lifetimes and unassigned generic type parameters.
///
/// - Non-static lifetimes: each occurrence gets a fresh positional name ('a, 'b, ...),
///   ignoring identity (two occurrences of `'x` become two distinct canonical names).
/// - Unassigned generic type parameters: renamed with fresh positional names (A, B, ...),
///   **preserving** identity (two occurrences of `T` both become `A`).
///
/// Only constructible via [`Type::canonicalize()`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CanonicalType(Type);

impl CanonicalType {
    /// Access the underlying `Type`.
    pub fn inner(&self) -> &Type {
        &self.0
    }

    /// Consume the wrapper and return the underlying `Type`.
    pub fn into_inner(self) -> Type {
        self.0
    }
}

impl AsRef<Type> for Type {
    fn as_ref(&self) -> &Type {
        self
    }
}

impl Type {
    /// The unit type `()`, represented as an empty tuple.
    pub const UNIT_TYPE: Type = Type::Tuple(Tuple { elements: vec![] });

    /// Returns `true` if `t` is a `Result` type.
    pub fn is_result(&self) -> bool {
        let Type::Path(t) = self else {
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
    pub fn bind_generic_type_parameters(&self, bindings: &HashMap<String, Type>) -> Type {
        match self {
            Type::Path(t) | Type::TypeAlias(t) => {
                let is_alias = matches!(self, Type::TypeAlias(_));
                let mut bound_generics = Vec::with_capacity(t.generic_arguments.len());
                for generic in &t.generic_arguments {
                    let bound_generic = match generic {
                        GenericArgument::TypeParameter(t) => {
                            GenericArgument::TypeParameter(t.bind_generic_type_parameters(bindings))
                        }
                        GenericArgument::Lifetime(_) | GenericArgument::Const(_) => {
                            generic.to_owned()
                        }
                    };
                    bound_generics.push(bound_generic);
                }
                let path = PathType {
                    package_id: t.package_id.clone(),
                    // Should we set this to `None`?
                    rustdoc_id: t.rustdoc_id,
                    base_type: t.base_type.clone(),
                    generic_arguments: bound_generics,
                };
                if is_alias {
                    Type::TypeAlias(path)
                } else {
                    Type::Path(path)
                }
            }
            Type::Reference(r) => Type::Reference(TypeReference {
                is_mutable: r.is_mutable,
                inner: Box::new(r.inner.bind_generic_type_parameters(bindings)),
                lifetime: r.lifetime.clone(),
            }),
            Type::Tuple(t) => {
                let mut bound_elements = Vec::with_capacity(t.elements.len());
                for inner in &t.elements {
                    bound_elements.push(inner.bind_generic_type_parameters(bindings));
                }
                Type::Tuple(Tuple {
                    elements: bound_elements,
                })
            }
            Type::ScalarPrimitive(s) => Type::ScalarPrimitive(s.clone()),
            Type::Slice(s) => Type::Slice(Slice {
                element_type: Box::new(s.element_type.bind_generic_type_parameters(bindings)),
            }),
            Type::Array(a) => Type::Array(Array {
                element_type: Box::new(a.element_type.bind_generic_type_parameters(bindings)),
                len: a.len,
            }),
            Type::RawPointer(r) => Type::RawPointer(RawPointer {
                is_mutable: r.is_mutable,
                inner: Box::new(r.inner.bind_generic_type_parameters(bindings)),
            }),
            Type::FunctionPointer(fp) => Type::FunctionPointer(FunctionPointer {
                inputs: fp
                    .inputs
                    .iter()
                    .map(|input| FunctionPointerInput {
                        name: input.name.clone(),
                        type_: input.type_.bind_generic_type_parameters(bindings),
                    })
                    .collect(),
                output: fp
                    .output
                    .as_ref()
                    .map(|t| Box::new(t.bind_generic_type_parameters(bindings))),
                abi: fp.abi.clone(),
                is_unsafe: fp.is_unsafe,
            }),
            Type::Generic(g) => {
                if let Some(bound_type) = bindings.get(&g.name) {
                    bound_type.clone()
                } else {
                    Type::Generic(g.to_owned())
                }
            }
        }
    }

    /// Check if a type can be used as a "template"—i.e. if it has any unassigned generic parameters.
    pub fn is_a_template(&self) -> bool {
        match self {
            Type::Path(path) | Type::TypeAlias(path) => {
                path.generic_arguments.iter().any(|arg| match arg {
                    GenericArgument::TypeParameter(g) => g.is_a_template(),
                    GenericArgument::Lifetime(
                        GenericLifetimeParameter::Static
                        | GenericLifetimeParameter::Named(_)
                        | GenericLifetimeParameter::Inferred,
                    ) => false,
                    GenericArgument::Const(_) => false,
                })
            }
            Type::Reference(r) => r.inner.is_a_template(),
            Type::Tuple(t) => t.elements.iter().any(|t| t.is_a_template()),
            Type::ScalarPrimitive(_) => false,
            Type::Slice(s) => s.element_type.is_a_template(),
            Type::Array(a) => a.element_type.is_a_template(),
            Type::RawPointer(r) => r.inner.is_a_template(),
            Type::FunctionPointer(fp) => {
                fp.inputs.iter().any(|input| input.type_.is_a_template())
                    || fp.output.as_ref().is_some_and(|t| t.is_a_template())
            }
            Type::Generic(_) => true,
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
            Type::Path(path) | Type::TypeAlias(path) => {
                for arg in &path.generic_arguments {
                    match arg {
                        GenericArgument::TypeParameter(g) => {
                            g._unassigned_generic_type_parameters(set);
                        }
                        GenericArgument::Lifetime(_) | GenericArgument::Const(_) => {}
                    }
                }
            }
            Type::Reference(r) => r.inner._unassigned_generic_type_parameters(set),
            Type::Tuple(t) => {
                for inner in &t.elements {
                    inner._unassigned_generic_type_parameters(set);
                }
            }
            Type::ScalarPrimitive(_) => {}
            Type::Slice(s) => s.element_type._unassigned_generic_type_parameters(set),
            Type::Array(a) => a.element_type._unassigned_generic_type_parameters(set),
            Type::RawPointer(r) => r.inner._unassigned_generic_type_parameters(set),
            Type::FunctionPointer(fp) => {
                for input in &fp.inputs {
                    input.type_._unassigned_generic_type_parameters(set);
                }
                if let Some(output) = &fp.output {
                    output._unassigned_generic_type_parameters(set);
                }
            }
            Type::Generic(t) => {
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
    pub fn is_a_template_for(&self, concrete_type: &Type) -> Option<HashMap<String, Type>> {
        let mut bindings = HashMap::new();
        if self._is_a_template_for(concrete_type, &mut bindings) {
            Some(bindings)
        } else {
            None
        }
    }

    pub(crate) fn _is_a_template_for(
        &self,
        concrete_type: &Type,
        bindings: &mut HashMap<String, Type>,
    ) -> bool {
        if concrete_type == self {
            return true;
        }
        use Type::*;
        match (concrete_type, self) {
            (Path(concrete_path), Path(templated_path))
            | (TypeAlias(concrete_path), TypeAlias(templated_path)) => {
                templated_path._is_a_resolved_path_type_template_for(concrete_path, bindings)
            }
            (Slice(concrete_slice), Slice(templated_slice)) => templated_slice
                .element_type
                ._is_a_template_for(&concrete_slice.element_type, bindings),
            (Array(concrete_array), Array(templated_array)) => {
                concrete_array.len == templated_array.len
                    && templated_array
                        .element_type
                        ._is_a_template_for(&concrete_array.element_type, bindings)
            }
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
            (RawPointer(concrete_ptr), RawPointer(templated_ptr)) => {
                concrete_ptr.is_mutable == templated_ptr.is_mutable
                    && templated_ptr
                        .inner
                        ._is_a_template_for(&concrete_ptr.inner, bindings)
            }
            (FunctionPointer(concrete_fp), FunctionPointer(templated_fp)) => {
                if concrete_fp.abi != templated_fp.abi
                    || concrete_fp.is_unsafe != templated_fp.is_unsafe
                {
                    return false;
                }
                if concrete_fp.inputs.len() != templated_fp.inputs.len() {
                    return false;
                }
                let inputs_match = concrete_fp
                    .inputs
                    .iter()
                    .zip(templated_fp.inputs.iter())
                    .all(|(concrete, templated)| {
                        templated
                            .type_
                            ._is_a_template_for(&concrete.type_, bindings)
                    });
                if !inputs_match {
                    return false;
                }
                match (&concrete_fp.output, &templated_fp.output) {
                    (Some(concrete_out), Some(templated_out)) => {
                        templated_out._is_a_template_for(concrete_out, bindings)
                    }
                    (None, None) => true,
                    _ => false,
                }
            }
            (_, Generic(parameter)) => {
                let previous = bindings.insert(parameter.name.clone(), concrete_type.clone());
                if let Some(previous) = previous
                    && &previous != concrete_type
                {
                    return false;
                }
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
    pub fn is_equivalent_to<'a, 'b>(
        &'a self,
        other: &'b Type,
    ) -> Option<HashMap<&'a str, &'b str>> {
        let mut self_id_gen = UnassignedIdGenerator::new();
        let mut other_id_gen = UnassignedIdGenerator::new();
        if self._is_equivalent_to(other, &mut self_id_gen, &mut other_id_gen) {
            Some(
                self_id_gen
                    .into_sorted_iter()
                    .zip(other_id_gen.into_sorted_iter())
                    .map(|((self_name, _), (other_name, _))| (self_name, other_name))
                    .collect(),
            )
        } else {
            None
        }
    }

    pub(crate) fn _is_equivalent_to<'a, 'b>(
        &'a self,
        other: &'b Type,
        self_id_gen: &mut UnassignedIdGenerator<'a>,
        other_id_gen: &mut UnassignedIdGenerator<'b>,
    ) -> bool {
        use Type::*;
        match (self, other) {
            (Path(self_path), Path(other_path)) | (TypeAlias(self_path), TypeAlias(other_path)) => {
                self_path._is_equivalent_to(other_path, self_id_gen, other_id_gen)
            }
            (Slice(self_slice), Slice(other_slice)) => self_slice.element_type._is_equivalent_to(
                &other_slice.element_type,
                self_id_gen,
                other_id_gen,
            ),
            (Array(self_array), Array(other_array)) => {
                self_array.len == other_array.len
                    && self_array.element_type._is_equivalent_to(
                        &other_array.element_type,
                        self_id_gen,
                        other_id_gen,
                    )
            }
            (Reference(self_reference), Reference(other_reference)) => self_reference
                .inner
                ._is_equivalent_to(&other_reference.inner, self_id_gen, other_id_gen),
            (Tuple(self_tuple), Tuple(other_tuple)) => {
                if self_tuple.elements.len() != other_tuple.elements.len() {
                    return false;
                }
                self_tuple
                    .elements
                    .iter()
                    .zip(other_tuple.elements.iter())
                    .all(|(self_type, other_type)| {
                        self_type._is_equivalent_to(other_type, self_id_gen, other_id_gen)
                    })
            }
            (ScalarPrimitive(self_p), ScalarPrimitive(other_p)) => self_p == other_p,
            (RawPointer(self_ptr), RawPointer(other_ptr)) => {
                self_ptr.is_mutable == other_ptr.is_mutable
                    && self_ptr
                        .inner
                        ._is_equivalent_to(&other_ptr.inner, self_id_gen, other_id_gen)
            }
            (FunctionPointer(self_fp), FunctionPointer(other_fp)) => {
                if self_fp.abi != other_fp.abi || self_fp.is_unsafe != other_fp.is_unsafe {
                    return false;
                }
                if self_fp.inputs.len() != other_fp.inputs.len() {
                    return false;
                }
                let inputs_match =
                    self_fp
                        .inputs
                        .iter()
                        .zip(other_fp.inputs.iter())
                        .all(|(s, o)| {
                            s.type_
                                ._is_equivalent_to(&o.type_, self_id_gen, other_id_gen)
                        });
                if !inputs_match {
                    return false;
                }
                match (&self_fp.output, &other_fp.output) {
                    (Some(s), Some(o)) => s._is_equivalent_to(o, self_id_gen, other_id_gen),
                    (None, None) => true,
                    _ => false,
                }
            }
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
            Type::Path(path) | Type::TypeAlias(path) => {
                path.generic_arguments.iter().any(|arg| match arg {
                    GenericArgument::TypeParameter(g) => g.has_implicit_lifetime_parameters(),
                    GenericArgument::Lifetime(GenericLifetimeParameter::Inferred) => true,
                    GenericArgument::Lifetime(
                        GenericLifetimeParameter::Named(_) | GenericLifetimeParameter::Static,
                    ) => false,
                    GenericArgument::Const(_) => false,
                })
            }
            Type::Reference(r) => {
                match &r.lifetime {
                    Lifetime::Inferred => {
                        return true;
                    }
                    Lifetime::Elided => {
                        return true;
                    }
                    Lifetime::Named(_) | Lifetime::Static => {}
                }
                r.inner.has_implicit_lifetime_parameters()
            }
            Type::Tuple(t) => t
                .elements
                .iter()
                .any(|t| t.has_implicit_lifetime_parameters()),
            Type::ScalarPrimitive(_) => false,
            Type::Slice(s) => s.element_type.has_implicit_lifetime_parameters(),
            Type::Array(a) => a.element_type.has_implicit_lifetime_parameters(),
            Type::RawPointer(r) => r.inner.has_implicit_lifetime_parameters(),
            Type::FunctionPointer(fp) => {
                fp.inputs
                    .iter()
                    .any(|input| input.type_.has_implicit_lifetime_parameters())
                    || fp
                        .output
                        .as_ref()
                        .is_some_and(|t| t.has_implicit_lifetime_parameters())
            }
            Type::Generic(_) => false,
        }
    }

    /// Replace all implicit lifetimes (e.g. `&'_ str` or the elided lifetime in `&str`) to
    /// the provided named lifetime.
    pub fn set_implicit_lifetimes(&mut self, inferred_lifetime: String) {
        match self {
            Type::Path(path) | Type::TypeAlias(path) => {
                for arg in path.generic_arguments.iter_mut() {
                    match arg {
                        GenericArgument::Lifetime(lifetime) => {
                            if matches!(lifetime, GenericLifetimeParameter::Inferred) {
                                *lifetime =
                                    GenericLifetimeParameter::from_name(inferred_lifetime.clone());
                            }
                        }
                        GenericArgument::TypeParameter(t) => {
                            t.set_implicit_lifetimes(inferred_lifetime.clone());
                        }
                        GenericArgument::Const(_) => {}
                    }
                }
            }
            Type::Reference(r) => {
                match &r.lifetime {
                    Lifetime::Inferred => {
                        r.lifetime = Lifetime::from_name(inferred_lifetime.clone());
                    }
                    Lifetime::Elided => {
                        r.lifetime = Lifetime::from_name(inferred_lifetime.clone());
                    }
                    Lifetime::Static | Lifetime::Named(_) => {}
                }
                r.inner.set_implicit_lifetimes(inferred_lifetime);
            }
            Type::Tuple(t) => t
                .elements
                .iter_mut()
                .for_each(|e| e.set_implicit_lifetimes(inferred_lifetime.clone())),
            Type::Slice(s) => s.element_type.set_implicit_lifetimes(inferred_lifetime),
            Type::Array(a) => a.element_type.set_implicit_lifetimes(inferred_lifetime),
            Type::RawPointer(r) => r.inner.set_implicit_lifetimes(inferred_lifetime),
            Type::FunctionPointer(fp) => {
                for input in fp.inputs.iter_mut() {
                    input
                        .type_
                        .set_implicit_lifetimes(inferred_lifetime.clone());
                }
                if let Some(output) = fp.output.as_mut() {
                    output.set_implicit_lifetimes(inferred_lifetime);
                }
            }
            Type::Generic(_) | Type::ScalarPrimitive(_) => {}
        }
    }

    /// Rename named lifetime parameters in this type according to the provided mapping.
    ///
    /// You don't need to provide a mapping for lifetimes that you don't want to rename.
    pub fn rename_lifetime_parameters(&mut self, original2renamed: &IndexMap<String, String>) {
        match self {
            Type::Path(t) | Type::TypeAlias(t) => {
                for arg in t.generic_arguments.iter_mut() {
                    match arg {
                        GenericArgument::TypeParameter(tp) => {
                            tp.rename_lifetime_parameters(original2renamed);
                        }
                        GenericArgument::Lifetime(l) => {
                            if let GenericLifetimeParameter::Named(named) = l
                                && let Some(new_name) = original2renamed.get(named.as_str())
                            {
                                *l = GenericLifetimeParameter::from_name(new_name.clone());
                            }
                        }
                        GenericArgument::Const(_) => {}
                    }
                }
            }
            Type::Reference(r) => {
                match &r.lifetime {
                    Lifetime::Named(l) => {
                        if let Some(new_name) = original2renamed.get(l.as_str()) {
                            r.lifetime = Lifetime::from_name(new_name.clone());
                        }
                    }
                    Lifetime::Static | Lifetime::Elided | Lifetime::Inferred => {}
                }
                r.inner.rename_lifetime_parameters(original2renamed);
            }
            Type::Tuple(t) => {
                for e in t.elements.iter_mut() {
                    e.rename_lifetime_parameters(original2renamed);
                }
            }
            Type::Slice(s) => {
                s.element_type.rename_lifetime_parameters(original2renamed);
            }
            Type::Array(a) => {
                a.element_type.rename_lifetime_parameters(original2renamed);
            }
            Type::RawPointer(r) => {
                r.inner.rename_lifetime_parameters(original2renamed);
            }
            Type::FunctionPointer(fp) => {
                for input in fp.inputs.iter_mut() {
                    input.type_.rename_lifetime_parameters(original2renamed);
                }
                if let Some(output) = fp.output.as_mut() {
                    output.rename_lifetime_parameters(original2renamed);
                }
            }
            Type::Generic(_) | Type::ScalarPrimitive(_) => {}
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
            Type::Path(path) | Type::TypeAlias(path) => {
                for arg in &path.generic_arguments {
                    match arg {
                        GenericArgument::TypeParameter(g) => {
                            g._lifetime_parameters(set);
                        }
                        GenericArgument::Lifetime(l) => {
                            set.insert(l.clone().into());
                        }
                        GenericArgument::Const(_) => {}
                    }
                }
            }
            Type::Reference(r) => {
                set.insert(r.lifetime.clone());
                r.inner._lifetime_parameters(set)
            }
            Type::Tuple(t) => {
                for inner in &t.elements {
                    inner._lifetime_parameters(set);
                }
            }
            Type::Slice(s) => s.element_type._lifetime_parameters(set),
            Type::Array(a) => a.element_type._lifetime_parameters(set),
            Type::RawPointer(r) => r.inner._lifetime_parameters(set),
            Type::FunctionPointer(fp) => {
                for input in &fp.inputs {
                    input.type_._lifetime_parameters(set);
                }
                if let Some(output) = &fp.output {
                    output._lifetime_parameters(set);
                }
            }
            Type::ScalarPrimitive(_) | Type::Generic(_) => {}
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
            Type::Path(path) | Type::TypeAlias(path) => {
                for arg in &path.generic_arguments {
                    match arg {
                        GenericArgument::TypeParameter(g) => {
                            g._named_lifetime_parameters(set);
                        }
                        GenericArgument::Lifetime(
                            GenericLifetimeParameter::Static | GenericLifetimeParameter::Inferred,
                        ) => {}
                        GenericArgument::Lifetime(GenericLifetimeParameter::Named(l)) => {
                            set.insert(l.as_str().to_owned());
                        }
                        GenericArgument::Const(_) => {}
                    }
                }
            }
            Type::Reference(r) => {
                match &r.lifetime {
                    Lifetime::Named(l) => {
                        set.insert(l.as_str().to_owned());
                    }
                    Lifetime::Static | Lifetime::Elided | Lifetime::Inferred => {}
                }
                r.inner._named_lifetime_parameters(set)
            }
            Type::Tuple(t) => {
                for inner in &t.elements {
                    inner._named_lifetime_parameters(set);
                }
            }
            Type::Slice(s) => s.element_type._named_lifetime_parameters(set),
            Type::Array(a) => a.element_type._named_lifetime_parameters(set),
            Type::RawPointer(r) => r.inner._named_lifetime_parameters(set),
            Type::FunctionPointer(fp) => {
                for input in &fp.inputs {
                    input.type_._named_lifetime_parameters(set);
                }
                if let Some(output) = &fp.output {
                    output._named_lifetime_parameters(set);
                }
            }
            Type::ScalarPrimitive(_) | Type::Generic(_) => {}
        }
    }

    /// Returns a canonicalized copy of this type, wrapped in [`CanonicalType`].
    ///
    /// - Non-static lifetimes: each occurrence gets a fresh canonical name ('a, 'b, 'c, ...),
    ///   regardless of whether it shares a name with another occurrence.
    /// - Unassigned generic type parameters: renamed with fresh positional names (A, B, ...),
    ///   **preserving** identity (two occurrences of `T` both become `A`).
    /// - Static lifetimes and scalar primitives are preserved as-is.
    /// - For [`Type::Path`] and [`Type::TypeAlias`], when a `rustdoc_id` is available, the
    ///   `base_type` is rewritten to the canonical shortest importable path reported by
    ///   the supplied [`CanonicalPathResolver`]. This ensures that the same type reached
    ///   via different exports canonicalizes to the same value.
    pub fn canonicalize(&self, resolver: &dyn CanonicalPathResolver) -> CanonicalType {
        let mut lifetime_counter = 0usize;
        let mut generic_counter = 0usize;
        let mut generic_name_map: HashMap<String, String> = HashMap::new();
        CanonicalType(self._canonicalize(
            resolver,
            &mut lifetime_counter,
            &mut generic_counter,
            &mut generic_name_map,
        ))
    }

    fn _canonicalize(
        &self,
        resolver: &dyn CanonicalPathResolver,
        lifetime_counter: &mut usize,
        generic_counter: &mut usize,
        generic_name_map: &mut HashMap<String, String>,
    ) -> Self {
        fn next_lifetime_name(counter: &mut usize) -> String {
            // Produce "a", "b", ..., "z", "aa", "ab", ...
            let mut n = *counter;
            *counter += 1;
            let mut name = String::new();
            loop {
                name.insert(0, (b'a' + (n % 26) as u8) as char);
                if n < 26 {
                    break;
                }
                n = n / 26 - 1;
            }
            name
        }

        fn next_generic_name(counter: &mut usize) -> String {
            // Produce "A", "B", ..., "Z", "AA", "AB", ...
            let mut n = *counter;
            *counter += 1;
            let mut name = String::new();
            loop {
                name.insert(0, (b'A' + (n % 26) as u8) as char);
                if n < 26 {
                    break;
                }
                n = n / 26 - 1;
            }
            name
        }

        fn canonicalize_lifetime(lifetime: &Lifetime, counter: &mut usize) -> Lifetime {
            match lifetime {
                Lifetime::Static => Lifetime::Static,
                Lifetime::Named(_) | Lifetime::Elided | Lifetime::Inferred => {
                    Lifetime::Named(NamedLifetime::new(next_lifetime_name(counter)))
                }
            }
        }

        fn canonicalize_generic_lifetime(
            lifetime: &GenericLifetimeParameter,
            counter: &mut usize,
        ) -> GenericLifetimeParameter {
            match lifetime {
                GenericLifetimeParameter::Static => GenericLifetimeParameter::Static,
                GenericLifetimeParameter::Named(_) | GenericLifetimeParameter::Inferred => {
                    GenericLifetimeParameter::Named(NamedLifetime::new(next_lifetime_name(counter)))
                }
            }
        }

        match self {
            Type::Path(t) | Type::TypeAlias(t) => {
                let is_alias = matches!(self, Type::TypeAlias(_));
                let generic_arguments = t
                    .generic_arguments
                    .iter()
                    .map(|arg| match arg {
                        GenericArgument::TypeParameter(inner) => {
                            GenericArgument::TypeParameter(inner._canonicalize(
                                resolver,
                                lifetime_counter,
                                generic_counter,
                                generic_name_map,
                            ))
                        }
                        GenericArgument::Lifetime(l) => GenericArgument::Lifetime(
                            canonicalize_generic_lifetime(l, lifetime_counter),
                        ),
                        GenericArgument::Const(_) => arg.clone(),
                    })
                    .collect();
                let base_type = if let Some(rustdoc_id) = t.rustdoc_id {
                    let id = GlobalItemId::new(rustdoc_id, t.package_id.clone());
                    resolver
                        .canonical_path(&id)
                        .unwrap_or_else(|| t.base_type.clone())
                } else {
                    t.base_type.clone()
                };
                let path = PathType {
                    package_id: t.package_id.clone(),
                    rustdoc_id: t.rustdoc_id,
                    base_type,
                    generic_arguments,
                };
                if is_alias {
                    Type::TypeAlias(path)
                } else {
                    Type::Path(path)
                }
            }
            Type::Reference(r) => Type::Reference(TypeReference {
                is_mutable: r.is_mutable,
                lifetime: canonicalize_lifetime(&r.lifetime, lifetime_counter),
                inner: Box::new(r.inner._canonicalize(
                    resolver,
                    lifetime_counter,
                    generic_counter,
                    generic_name_map,
                )),
            }),
            Type::Tuple(t) => Type::Tuple(Tuple {
                elements: t
                    .elements
                    .iter()
                    .map(|e| {
                        e._canonicalize(
                            resolver,
                            lifetime_counter,
                            generic_counter,
                            generic_name_map,
                        )
                    })
                    .collect(),
            }),
            Type::Slice(s) => Type::Slice(Slice {
                element_type: Box::new(s.element_type._canonicalize(
                    resolver,
                    lifetime_counter,
                    generic_counter,
                    generic_name_map,
                )),
            }),
            Type::Array(a) => Type::Array(Array {
                element_type: Box::new(a.element_type._canonicalize(
                    resolver,
                    lifetime_counter,
                    generic_counter,
                    generic_name_map,
                )),
                len: a.len,
            }),
            Type::RawPointer(r) => Type::RawPointer(RawPointer {
                is_mutable: r.is_mutable,
                inner: Box::new(r.inner._canonicalize(
                    resolver,
                    lifetime_counter,
                    generic_counter,
                    generic_name_map,
                )),
            }),
            Type::FunctionPointer(fp) => Type::FunctionPointer(FunctionPointer {
                inputs: fp
                    .inputs
                    .iter()
                    .map(|input| FunctionPointerInput {
                        name: None,
                        type_: input.type_._canonicalize(
                            resolver,
                            lifetime_counter,
                            generic_counter,
                            generic_name_map,
                        ),
                    })
                    .collect(),
                output: fp.output.as_ref().map(|t| {
                    Box::new(t._canonicalize(
                        resolver,
                        lifetime_counter,
                        generic_counter,
                        generic_name_map,
                    ))
                }),
                abi: fp.abi.clone(),
                is_unsafe: fp.is_unsafe,
            }),
            Type::ScalarPrimitive(_) => self.clone(),
            Type::Generic(g) => {
                let canonical_name = generic_name_map
                    .entry(g.name.clone())
                    .or_insert_with(|| next_generic_name(generic_counter))
                    .clone();
                Type::Generic(Generic {
                    name: canonical_name,
                })
            }
        }
    }
}

impl Debug for Type {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Path(t) | Type::TypeAlias(t) => write!(f, "{t:?}"),
            Type::Reference(r) => write!(f, "{r:?}"),
            Type::Tuple(t) => write!(f, "{t:?}"),
            Type::ScalarPrimitive(s) => write!(f, "{s:?}"),
            Type::Slice(s) => write!(f, "{s:?}"),
            Type::Array(a) => write!(f, "{a:?}"),
            Type::RawPointer(r) => write!(f, "{r:?}"),
            Type::FunctionPointer(fp) => write!(f, "{fp:?}"),
            Type::Generic(g) => write!(f, "{g:?}"),
        }
    }
}

impl From<Tuple> for Type {
    fn from(value: Tuple) -> Self {
        Self::Tuple(value)
    }
}

impl From<PathType> for Type {
    fn from(value: PathType) -> Self {
        Self::Path(value)
    }
}

impl From<TypeReference> for Type {
    fn from(value: TypeReference) -> Self {
        Self::Reference(value)
    }
}

impl From<RawPointer> for Type {
    fn from(value: RawPointer) -> Self {
        Self::RawPointer(value)
    }
}

impl From<Array> for Type {
    fn from(value: Array) -> Self {
        Self::Array(value)
    }
}

impl From<FunctionPointer> for Type {
    fn from(value: FunctionPointer) -> Self {
        Self::FunctionPointer(value)
    }
}

#[cfg(test)]
mod tests {
    use crate::{GenericLifetimeParameter, Lifetime, NamedLifetime};

    #[test]
    fn named_lifetimes_are_structurally_compared() {
        // Different names are not equal.
        assert_ne!(
            Lifetime::Named(NamedLifetime::new("a")),
            Lifetime::Named(NamedLifetime::new("b")),
        );
        // Named is not equal to Elided or Inferred.
        assert_ne!(Lifetime::Named(NamedLifetime::new("a")), Lifetime::Elided,);
        assert_ne!(Lifetime::Named(NamedLifetime::new("a")), Lifetime::Inferred,);
        // Same name is equal.
        assert_eq!(
            Lifetime::Named(NamedLifetime::new("a")),
            Lifetime::Named(NamedLifetime::new("a")),
        );
    }

    #[test]
    fn named_generic_lifetimes_are_structurally_compared() {
        assert_ne!(
            GenericLifetimeParameter::Named(NamedLifetime::new("a")),
            GenericLifetimeParameter::Named(NamedLifetime::new("b")),
        );
        assert_eq!(
            GenericLifetimeParameter::Named(NamedLifetime::new("a")),
            GenericLifetimeParameter::Named(NamedLifetime::new("a")),
        );
    }
}
