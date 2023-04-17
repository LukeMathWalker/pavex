use std::borrow::Cow;

use ahash::HashMap;
use indexmap::IndexSet;

pub(crate) use match_result::{MatchResult, MatchResultVariant};

use crate::language::{Callable, ResolvedType};

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
    Callable(Cow<'a, Callable>),
    /// A branching constructor: extract either the `Ok(T)` or the `Err(E)` variant out of a
    /// [`Result<T,E>`](Result).
    MatchResult(Cow<'a, MatchResult>),
    /// A type that is provided by the framework.
    ///
    /// You could say that it's a bit of a hack to list this as a "computation".
    /// Open to suggestions on how to model this better!
    FrameworkItem(Cow<'a, ResolvedType>),
}

impl<'a> Computation<'a> {
    #[allow(unused)]
    pub fn ref_(&self) -> Computation<'_> {
        match self {
            Computation::Callable(c) => Computation::Callable(Cow::Borrowed(c)),
            Computation::MatchResult(m) => Computation::MatchResult(Cow::Borrowed(m)),
            Computation::FrameworkItem(i) => Computation::FrameworkItem(Cow::Borrowed(i)),
        }
    }

    /// Return a new computation with the same behaviour but with a `'static` lifetime.
    ///
    /// This can be useful to break a reference relationship between the computation data and
    /// the container they are being borrowed from.
    pub fn into_owned(self) -> Computation<'static> {
        match self {
            Computation::Callable(c) => Computation::Callable(Cow::Owned(c.into_owned())),
            Computation::MatchResult(c) => Computation::MatchResult(Cow::Owned(c.into_owned())),
            Computation::FrameworkItem(c) => Computation::FrameworkItem(Cow::Owned(c.into_owned())),
        }
    }

    /// The types required as input parameters by this computation.
    pub fn input_types(&self) -> Cow<[ResolvedType]> {
        match self {
            Computation::Callable(c) => Cow::Borrowed(c.inputs.as_slice()),
            Computation::MatchResult(m) => Cow::Owned(vec![m.input.clone()]),
            Computation::FrameworkItem(_) => Cow::Owned(vec![]),
        }
    }

    /// The type returned by this computation.
    ///
    /// This is `None` for computations that don't return a value.
    pub fn output_type(&self) -> Option<&crate::language::ResolvedType> {
        match self {
            Computation::Callable(c) => c.output.as_ref(),
            Computation::MatchResult(m) => Some(&m.output),
            Computation::FrameworkItem(i) => Some(i.as_ref()),
        }
    }

    /// Replace all unassigned generic type parameters in this computation with the
    /// concrete types specified in `bindings`.
    ///
    /// The newly "bound" computation will be returned.
    pub fn bind_generic_type_parameters(
        &self,
        bindings: &HashMap<String, ResolvedType>,
    ) -> Computation<'_> {
        match self {
            Computation::Callable(c) => {
                Computation::Callable(Cow::Owned(c.bind_generic_type_parameters(bindings)))
            }
            Computation::MatchResult(m) => {
                Computation::MatchResult(Cow::Owned(m.bind_generic_type_parameters(bindings)))
            }
            Computation::FrameworkItem(i) => {
                Computation::FrameworkItem(Cow::Owned(i.bind_generic_type_parameters(bindings)))
            }
        }
    }

    /// Returns the set of all unassigned generic type parameters in this computation.
    #[allow(unused)]
    pub(crate) fn unassigned_generic_type_parameters(&self) -> IndexSet<String> {
        match self {
            Computation::Callable(c) => c.unassigned_generic_type_parameters(),
            Computation::MatchResult(m) => m.unassigned_generic_type_parameters(),
            Computation::FrameworkItem(i) => i.unassigned_generic_type_parameters(),
        }
    }
}

impl<'a> From<Callable> for Computation<'a> {
    fn from(value: Callable) -> Self {
        Self::Callable(Cow::Owned(value))
    }
}

impl<'a> From<MatchResult> for Computation<'a> {
    fn from(value: MatchResult) -> Self {
        Self::MatchResult(Cow::Owned(value))
    }
}

impl<'a> From<&'a Callable> for Computation<'a> {
    fn from(value: &'a Callable) -> Self {
        Self::Callable(Cow::Borrowed(value))
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

impl<'a> From<Cow<'a, MatchResult>> for Computation<'a> {
    fn from(value: Cow<'a, MatchResult>) -> Self {
        Self::MatchResult(value)
    }
}
