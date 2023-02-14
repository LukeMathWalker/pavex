use crate::language::{ResolvedType, TypeReference};

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
}
