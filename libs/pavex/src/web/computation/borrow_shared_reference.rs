use ahash::HashMap;
use indexmap::IndexSet;

use crate::language::{NamedTypeGeneric, ResolvedType, TypeReference};

/// Borrow a shared reference for a type - i.e. get a `&T` from a `T`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct BorrowSharedReference {
    pub(crate) input: ResolvedType,
    pub(crate) output: ResolvedType,
}

impl BorrowSharedReference {
    pub fn new(input: ResolvedType) -> Self {
        let output = ResolvedType::Reference(TypeReference {
            is_mutable: false,
            is_static: false,
            inner: Box::new(input.clone()),
        });
        Self { input, output }
    }

    /// Replace all unassigned generic type parameters in this reference with the
    /// concrete types specified in `bindings`.
    ///
    /// The newly "bound" reference will be returned.
    pub fn bind_generic_type_parameters(
        &self,
        bindings: &HashMap<NamedTypeGeneric, ResolvedType>,
    ) -> Self {
        Self {
            input: self.input.bind_generic_type_parameters(bindings),
            output: self.output.bind_generic_type_parameters(bindings),
        }
    }

    /// Returns the set of all unassigned generic type parameters in this borrow.
    pub(crate) fn unassigned_generic_type_parameters(&self) -> IndexSet<NamedTypeGeneric> {
        let mut result = IndexSet::new();
        result.extend(self.input.unassigned_generic_type_parameters());
        result.extend(self.output.unassigned_generic_type_parameters());
        result
    }
}
