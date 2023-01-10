use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

use guppy::graph::PackageGraph;

use pavex_builder::{AppBlueprint, RawCallableIdentifiers};

use crate::diagnostic;
use crate::diagnostic::{CompilerDiagnostic, LocationExt, SourceSpanExt};
use crate::language::{Callable, ResolvedPath, ResolvedType};

/// A transformation that, given a set of inputs, **constructs** a new type.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum Constructor {
    /// An inline transformation: construct a `&T` from a `T`.
    BorrowSharedReference(BorrowSharedReference),
    /// A branching constructor: extract either the `Ok(T)` or the `Err(E)` variant out of a
    /// [`Result<T,E>`](core::result::Result).
    MatchResult(MatchResult),
    /// Build a new instance of a type by calling a function or a method.
    ///
    /// The constructor can take zero or more arguments as inputs.
    /// It must return a non-unit output type.
    Callable(Callable),
}

/// Borrow a shared reference for a type - i.e. get a `&T` from a `T`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct BorrowSharedReference {
    pub(crate) input: ResolvedType,
    pub(crate) output: ResolvedType,
}

/// A branching constructor: extract one of the variant out of a Rust enum.
/// E.g. get a `T` (or `E`) from a `Result<T, E>`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct MatchResult {
    pub(crate) input: ResolvedType,
    pub(crate) output: ResolvedType,
    pub(crate) variant: MatchResultVariant,
}

#[derive(Debug, Clone, PartialEq, Eq, Copy, Hash)]
pub(crate) enum MatchResultVariant {
    Ok,
    Err,
}

impl TryFrom<Callable> for Constructor {
    type Error = ConstructorValidationError;

    fn try_from(c: Callable) -> Result<Self, Self::Error> {
        if c.output.is_none() {
            return Err(ConstructorValidationError::CannotReturnTheUnitType(c.path));
        }
        Ok(Constructor::Callable(c))
    }
}

impl Constructor {
    /// Create a new borrow-ing constructor.
    /// It will return a `&input` when invoked.
    pub fn shared_borrow(input: ResolvedType) -> Self {
        let output = ResolvedType {
            is_shared_reference: true,
            ..input.clone()
        };
        Self::BorrowSharedReference(BorrowSharedReference { input, output })
    }

    /// Return a new match-ing constructor for the `Ok(T)` and the `Err(E)` variant of a `Result`.
    ///
    /// It panics if `enum_type` is not an enum.
    pub fn match_result(result_type: &ResolvedType) -> ResultMatch {
        assert_eq!(
            result_type.generic_arguments.len(),
            2,
            "{result_type:?} does not have two generic arguments, as expected"
        );
        let mut generics = result_type.generic_arguments.iter();
        let ok_type = generics.next().unwrap().to_owned();
        let ok_constructor = MatchResult {
            input: result_type.to_owned(),
            output: ok_type,
            variant: MatchResultVariant::Ok,
        };
        let err_type = generics.next().unwrap().to_owned();
        let err_constructor = MatchResult {
            input: result_type.to_owned(),
            output: err_type,
            variant: MatchResultVariant::Err,
        };
        ResultMatch {
            ok: Constructor::MatchResult(ok_constructor),
            err: Constructor::MatchResult(err_constructor),
        }
    }

    /// The type returned by the constructor.
    pub fn output_type(&self) -> &ResolvedType {
        match self {
            Constructor::BorrowSharedReference(s) => &s.output,
            Constructor::Callable(c) => c.output.as_ref().unwrap(),
            Constructor::MatchResult(m) => &m.output,
        }
    }

    pub fn input_types(&self) -> Cow<[ResolvedType]> {
        match self {
            Constructor::BorrowSharedReference(r) => Cow::Owned(vec![r.input.clone()]),
            Constructor::Callable(c) => Cow::Borrowed(c.inputs.as_slice()),
            Constructor::MatchResult(m) => Cow::Owned(vec![m.input.clone()]),
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub(crate) enum ConstructorValidationError {
    #[error("I expect all constructors to return *something*.\nThis constructor doesn't, it returns the unit type - `()`.")]
    CannotReturnTheUnitType(ResolvedPath),
}

impl ConstructorValidationError {
    pub fn into_diagnostic(
        self,
        resolved_paths2identifiers: &HashMap<ResolvedPath, HashSet<RawCallableIdentifiers>>,
        app_blueprint: &AppBlueprint,
        package_graph: &PackageGraph,
    ) -> Result<CompilerDiagnostic, miette::Error> {
        match self {
            Self::CannotReturnTheUnitType(ref constructor_path) => {
                let raw_identifier = resolved_paths2identifiers[constructor_path]
                    .iter()
                    .next()
                    .unwrap();
                let location = &app_blueprint.constructor_locations[raw_identifier];
                let source = location.source_file(package_graph)?;
                let label = diagnostic::get_f_macro_invocation_span(&source, location)
                    .map(|s| s.labeled("The constructor was registered here".into()));
                let diagnostic = CompilerDiagnostic::builder(source, self)
                    .optional_label(label)
                    .build();
                Ok(diagnostic)
            }
        }
    }
}

/// The `Ok` and `Err` constructors returned by [`Constructor::match_result`].
pub(crate) struct ResultMatch {
    pub(crate) ok: Constructor,
    pub(crate) err: Constructor,
}
