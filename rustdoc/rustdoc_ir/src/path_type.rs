use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};

use guppy::PackageId;

use crate::generics_equivalence::UnassignedIdGenerator;
use crate::render::{deserialize_package_id, serialize_package_id};
use crate::{GenericArgument, ResolvedType};

/// A named type identified by its fully-qualified path—e.g. `std::vec::Vec<u32>`.
#[derive(serde::Serialize, serde::Deserialize, Eq, Clone)]
pub struct PathType {
    #[serde(serialize_with = "serialize_package_id")]
    #[serde(deserialize_with = "deserialize_package_id")]
    // `PackageId` doesn't implement serde::Deserialize/serde::Serialize, therefore we must
    // manually specify deserializer and serializer to make the whole `ResolvedPathType`
    // (de)serializable.
    /// The id of the package that defines this type.
    pub package_id: PackageId,
    /// The id associated with this type within the (JSON) docs for `package_id`.
    ///
    /// The id is optional to allow for flexible usage patterns—e.g. to leverage [`ResolvedType`]
    /// to work with types that we want to code-generate into a new crate.
    pub rustdoc_id: Option<rustdoc_types::Id>,
    /// The fully-qualified path segments for this type, e.g. `["std", "vec", "Vec"]`.
    pub base_type: Vec<String>,
    /// The generic arguments applied to this type, e.g. `[u32]` in `Vec<u32>`.
    pub generic_arguments: Vec<GenericArgument>,
}

impl PathType {
    pub(crate) fn _is_a_resolved_path_type_template_for(
        &self,
        concrete_type: &PathType,
        bindings: &mut ahash::HashMap<String, ResolvedType>,
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
                    if let Some(previous_assignment) = previous_assignment
                        && &previous_assignment != assigned
                    {
                        tracing::trace!(
                            "Type parameter `{:?}` was already assigned to `{:?}` but is now being assigned to `{:?}`",
                            unassigned,
                            previous_assignment,
                            assigned
                        );
                        return false;
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
