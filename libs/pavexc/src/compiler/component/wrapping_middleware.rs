use indexmap::IndexSet;

use crate::{language::{Callable, ResolvedType}, compiler::computation::MatchResult};
use std::borrow::Cow;

/// A callable that gets invoked during the processing of incoming requests
/// for one or more routes.
/// It must return a type that implements `pavex::response::IntoResponse`.
/// It can be fallible, as long as the `Ok` type implements `pavex::response::IntoResponse`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct WrappingMiddleware<'a> {
    pub(crate) callable: Cow<'a, Callable>,
}

impl<'a> WrappingMiddleware<'a> {
    pub fn new(c: Cow<'a, Callable>) -> Result<Self, WrappingMiddlewareValidationError> {
        use WrappingMiddlewareValidationError::*;

        let mut output_type = c.output.as_ref().ok_or(CannotReturnTheUnitType)?.clone();

        // If it is fallible, we make sure that it returns a non-unit type on the happy path.
        if output_type.is_result() {
            let m = MatchResult::match_result(&output_type);
            output_type = m.ok.output;
            if output_type == ResolvedType::UNIT_TYPE {
                return Err(CannotFalliblyReturnTheUnitType);
            }
        }

        let output_unassigned_generic_parameters = output_type.unassigned_generic_type_parameters();
        let mut free_parameters = IndexSet::new();
        for input in c.inputs.iter() {
            free_parameters.extend(
                input
                    .unassigned_generic_type_parameters()
                    .difference(&output_unassigned_generic_parameters)
                    .cloned(),
            );
        }
        if !free_parameters.is_empty() {
            return Err(UnderconstrainedGenericParameters {
                parameters: free_parameters,
            });
        }

        Ok(Self { callable: c })
    }

    pub fn output_type(&self) -> &ResolvedType {
        self.callable.output.as_ref().unwrap()
    }

    pub fn input_types(&self) -> &[ResolvedType] {
        self.callable.inputs.as_slice()
    }

    pub fn into_owned(self) -> WrappingMiddleware<'static> {
        WrappingMiddleware {
            callable: Cow::Owned(self.callable.into_owned()),
        }
    }
}

#[derive(thiserror::Error, Debug, Clone)]
pub(crate) enum WrappingMiddlewareValidationError {
    #[error(
        "All wrapping middlewares must return a type that can be converted into a \
        `pavex::response::Response`.\n\
        This middleware doesn't: it returns the unit type, `()`. I can't convert `()` into an HTTP response."
    )]
    CannotReturnTheUnitType,
    #[error(
        "All wrapping middlewares must return a type that can be converted into a \
        `pavex::response::Response`.\n\
        This middleware doesn't: it returns the unit type, `()`, when successful. I can't convert `()` into an HTTP response."
    )]
    CannotFalliblyReturnTheUnitType,
    #[error("Input parameters for a wrapping middleware can't have any *unassigned* generic type parameters that appear exclusively in its input parameters.")]
    UnderconstrainedGenericParameters { parameters: IndexSet<String> },
}
