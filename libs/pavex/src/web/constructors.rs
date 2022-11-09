use crate::language::{Callable, ResolvedPath, ResolvedType};

/// A transformation that, given a set of inputs, **constructs** a new type.
#[derive(Debug, Clone)]
pub(crate) enum Constructor {
    /// An inline transformation: construct a `&T` from a `T`.
    BorrowSharedReference(BorrowSharedReference),
    /// Build a new instance of a type by calling a function or a method.
    ///
    /// The constructor can take zero or more arguments as inputs.
    /// It must return a non-unit output type.
    Callable(Callable),
}

/// Borrow a shared reference for a type - i.e. get a `&T` from a `T`.
#[derive(Debug, Clone)]
pub(crate) struct BorrowSharedReference {
    pub(crate) input: ResolvedType,
    output: ResolvedType,
}

impl TryFrom<Callable> for Constructor {
    type Error = ConstructorValidationError;

    fn try_from(c: Callable) -> Result<Self, Self::Error> {
        if c.output.base_type == vec!["()"] {
            return Err(ConstructorValidationError::CannotReturnTheUnitType(
                c.path.to_owned(),
            ));
        }
        Ok(Constructor::Callable(c))
    }
}

impl Constructor {
    /// Create a new borrow-ing constructor.
    /// It will return a `&t` when invoked.
    pub fn shared_borrow(input: ResolvedType) -> Self {
        let output = ResolvedType {
            is_shared_reference: true,
            ..input.clone()
        };
        Self::BorrowSharedReference(BorrowSharedReference { input, output })
    }

    /// The type returned by the constructor.
    pub fn output_type(&self) -> &ResolvedType {
        match self {
            Constructor::BorrowSharedReference(s) => &s.output,
            Constructor::Callable(c) => &c.output,
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub(crate) enum ConstructorValidationError {
    #[error("I expect all constructors to return *something*.\nThis constructor doesn't, it returns the unit type - `()`.")]
    CannotReturnTheUnitType(ResolvedPath),
}
