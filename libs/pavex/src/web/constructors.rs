use std::borrow::Cow;

use crate::language::ResolvedType;
use crate::web::computation::{Computation, MatchResult};
use crate::web::utils::is_result;

/// Build a new instance of a type by performing a computation.
///
/// The constructor can take zero or more arguments as inputs.
/// It must return a non-unit output type.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct Constructor<'a>(pub(crate) Computation<'a>);

/// A [`Constructor`] that returns a `Result`.
/// The `Ok` variant must be a non-unit type.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct FallibleConstructor<'a>(Computation<'a>);

impl<'a> TryFrom<Computation<'a>> for Constructor<'a> {
    type Error = ConstructorValidationError;

    fn try_from(c: Computation<'a>) -> Result<Self, Self::Error> {
        if c.output_type().is_none() {
            return Err(ConstructorValidationError::CannotReturnTheUnitType);
        }
        let output_type = c.output_type().unwrap();
        // If the constructor is fallible, we make sure that it returns a non-unit type on
        // the happy path.
        if is_result(output_type) {
            let m = MatchResult::match_result(output_type);
            if m.ok.output == ResolvedType::UNIT_TYPE {
                return Err(ConstructorValidationError::CannotFalliblyReturnTheUnitType);
            }
        }
        Ok(Constructor(c))
    }
}

impl<'a> TryFrom<Constructor<'a>> for FallibleConstructor<'a> {
    type Error = FallibleConstructorValidationError;

    fn try_from(c: Constructor<'a>) -> Result<Self, Self::Error> {
        if !is_result(c.output_type()) {
            Err(FallibleConstructorValidationError::CannotBeInfallible)
        } else {
            Ok(Self(c.0))
        }
    }
}

impl<'a> From<Constructor<'a>> for Computation<'a> {
    fn from(value: Constructor<'a>) -> Self {
        value.0
    }
}

impl<'a> Constructor<'a> {
    /// The type returned by the constructor.
    pub fn output_type(&self) -> &ResolvedType {
        self.0.output_type().unwrap()
    }

    /// The inputs types used by this constructor.
    pub fn input_types(&self) -> Cow<[ResolvedType]> {
        self.0.input_types()
    }

    /// If the constructor is fallible, it returns a wrapper type that exposes additional methods
    /// to manipulate the `Err` and the `Ok` variant it returns.
    pub fn as_fallible(
        &self,
    ) -> Result<FallibleConstructor<'a>, FallibleConstructorValidationError> {
        self.clone().try_into()
    }

    pub fn into_owned(self) -> Constructor<'static> {
        Constructor(self.0.into_owned())
    }
}

impl<'a> FallibleConstructor<'a> {
    /// The type returned by the constructor.
    pub fn output_type(&self) -> &ResolvedType {
        self.0.output_type().unwrap()
    }

    /// Return a new match-ing constructor for the `Ok(T)` variant and a computation for the
    /// `Err(E)` variant of a `Result`.
    pub fn matchers(&self) -> ConstructorResultMatchers {
        let m = MatchResult::match_result(self.output_type());
        // This is certainly valid because we validate that the `Ok` variant is a not a unit type
        // when building a `Constructor`.
        let ok = Constructor(m.ok.into());
        ConstructorResultMatchers { ok, err: m.err }
    }
}

#[derive(thiserror::Error, Debug, Clone)]
pub(crate) enum ConstructorValidationError {
    #[error("All constructors must return *something*.\nThis constructor doesn't: it returns the unit type, `()`.")]
    CannotReturnTheUnitType,
    #[error("All fallible constructors must return *something* when successful.\nThis fallible constructor doesn't: it returns the unit type when successful, `Ok(())`.")]
    CannotFalliblyReturnTheUnitType,
}

#[derive(thiserror::Error, Debug, Clone)]
pub(crate) enum FallibleConstructorValidationError {
    #[error("Fallible constructors must be infallible.\nThis constructor isn't: it doesn't return a `Result`.")]
    CannotBeInfallible,
}

/// The `Ok` and `Err` `MatchResult`s returned by [`FallibleConstructor::matchers`].
/// The `Ok` variant is guaranteed to be a valid constructor - i.e. it doesn't return the unit
/// type.
pub(crate) struct ConstructorResultMatchers<'a> {
    pub(crate) ok: Constructor<'a>,
    pub(crate) err: MatchResult,
}
