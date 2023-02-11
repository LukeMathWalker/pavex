use std::borrow::Cow;

pub(crate) use borrow_shared_reference::BorrowSharedReference;
pub(crate) use match_result::{MatchResult, MatchResultVariant};

use crate::language::Callable;

mod borrow_shared_reference;
mod match_result;

/// A computation takes zero or more inputs and returns a single output.
///
/// It can be a function, a method, a match-arm that de-structures a `Result`
/// type, the action of borrowing a shared reference, etc.
///
/// You can think of it as a generalised function, even though not all computations
/// map to a function-like syntax in Rust (e.g. borrowing).
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub(crate) enum Computation<'a> {
    /// Build a new instance of a type by calling a function or a method.
    ///
    /// The constructor can take zero or more arguments as inputs.
    /// It must return a non-unit output type.
    Callable(Cow<'a, Callable>),
    /// A branching constructor: extract either the `Ok(T)` or the `Err(E)` variant out of a
    /// [`Result<T,E>`](Result).
    MatchResult(Cow<'a, MatchResult>),
    /// An inline transformation: construct a `&T` from a `T`.
    BorrowSharedReference(Cow<'a, BorrowSharedReference>),
}

impl<'a> Computation<'a> {
    pub fn ref_(&self) -> Computation<'_> {
        match self {
            Computation::Callable(c) => Computation::Callable(Cow::Borrowed(c)),
            Computation::MatchResult(m) => Computation::MatchResult(Cow::Borrowed(m)),
            Computation::BorrowSharedReference(b) => {
                Computation::BorrowSharedReference(Cow::Borrowed(b))
            }
        }
    }

    pub fn into_owned(self) -> Computation<'static> {
        match self {
            Computation::Callable(c) => Computation::Callable(Cow::Owned(c.into_owned())),
            Computation::MatchResult(c) => Computation::MatchResult(Cow::Owned(c.into_owned())),
            Computation::BorrowSharedReference(b) => {
                Computation::BorrowSharedReference(Cow::Owned(b.into_owned()))
            }
        }
    }

    pub fn input_types(&self) -> Cow<[crate::language::ResolvedType]> {
        match self {
            Computation::Callable(c) => Cow::Borrowed(c.inputs.as_slice()),
            Computation::MatchResult(m) => Cow::Owned(vec![m.input.clone()]),
            Computation::BorrowSharedReference(b) => Cow::Owned(vec![b.input.clone()]),
        }
    }

    pub fn output_type(&self) -> Option<&crate::language::ResolvedType> {
        match self {
            Computation::Callable(c) => c.output.as_ref(),
            Computation::MatchResult(m) => Some(&m.output),
            Computation::BorrowSharedReference(b) => Some(&b.output),
        }
    }
}

impl<'a> From<Callable> for Computation<'a> {
    fn from(value: Callable) -> Self {
        Self::Callable(Cow::Owned(value.into()))
    }
}

impl<'a> From<BorrowSharedReference> for Computation<'a> {
    fn from(value: BorrowSharedReference) -> Self {
        Self::BorrowSharedReference(Cow::Owned(value.into()))
    }
}

impl<'a> From<MatchResult> for Computation<'a> {
    fn from(value: MatchResult) -> Self {
        Self::MatchResult(Cow::Owned(value))
    }
}

impl<'a> From<&'a Callable> for Computation<'a> {
    fn from(value: &'a Callable) -> Self {
        Self::Callable(Cow::Borrowed(value.into()))
    }
}

impl<'a> From<&'a BorrowSharedReference> for Computation<'a> {
    fn from(value: &'a BorrowSharedReference) -> Self {
        Self::BorrowSharedReference(Cow::Borrowed(value.into()))
    }
}

impl<'a> From<&'a MatchResult> for Computation<'a> {
    fn from(value: &'a MatchResult) -> Self {
        Self::MatchResult(Cow::Borrowed(value))
    }
}

impl<'a> From<Cow<'a, Callable>> for Computation<'a> {
    fn from(value: Cow<'a, Callable>) -> Self {
        Self::Callable(value)
    }
}

impl<'a> From<Cow<'a, BorrowSharedReference>> for Computation<'a> {
    fn from(value: Cow<'a, BorrowSharedReference>) -> Self {
        Self::BorrowSharedReference(value)
    }
}

impl<'a> From<Cow<'a, MatchResult>> for Computation<'a> {
    fn from(value: Cow<'a, MatchResult>) -> Self {
        Self::MatchResult(value)
    }
}
