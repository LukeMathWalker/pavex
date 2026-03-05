use std::fmt::{Debug, Formatter};

use ahash::{HashMap, HashMapExt};
use indexmap::{IndexMap, IndexSet};

use crate::generics_equivalence::UnassignedIdGenerator;
use crate::{
    GenericArgument, GenericLifetimeParameter, Lifetime, PathType, ResolvedType, Slice, Tuple,
    TypeReference,
};

impl AsRef<ResolvedType> for ResolvedType {
    fn as_ref(&self) -> &ResolvedType {
        self
    }
}

impl ResolvedType {
    /// The unit type `()`, represented as an empty tuple.
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

    pub(crate) fn _is_a_template_for(
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
    pub fn is_equivalent_to<'a, 'b>(
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

    pub(crate) fn _is_equivalent_to<'a, 'b>(
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
                    if let GenericArgument::Lifetime(lifetime) = arg
                        && let GenericLifetimeParameter::Named(name) = lifetime
                        && name == "_"
                    {
                        *lifetime = GenericLifetimeParameter::Named(inferred_lifetime.clone());
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
                            if let GenericLifetimeParameter::Named(l) = l
                                && let Some(new_name) = original2renamed.get(l)
                            {
                                *l = new_name.clone();
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

    /// Format this type for display in user-facing error messages.
    pub fn display_for_error(&self) -> String {
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

    use crate::{GenericLifetimeParameter, Lifetime};

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
