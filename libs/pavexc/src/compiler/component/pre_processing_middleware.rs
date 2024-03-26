use indexmap::IndexSet;

use crate::{
    compiler::computation::MatchResult,
    language::{Callable, ResolvedType},
};
use std::borrow::Cow;

/// A callable that gets invoked during the processing of incoming requests
/// for one or more routes.
///
/// # Input parameters
///
/// There are no constraints on the input parameters of a pre-processing middleware.
///
/// # Output type
///
/// If infallible, the output type must be `pavex::middleware::Processing<T>`,
/// where `T` implements `pavex::response::IntoResponse`.  
/// If fallible, the output type must be a `Result<pavex::middleware::Processing<T>, E>` where `T` implements
/// `pavex::response::IntoResponse`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct PreProcessingMiddleware<'a> {
    pub(crate) callable: Cow<'a, Callable>,
}

impl<'a> PreProcessingMiddleware<'a> {
    /// Creates a new pre-processing middleware from a callable, either owned or borrowed.
    ///
    /// This function validates that the callable satisfies all the constraints of
    /// a pre-processing middleware. An error is returned if it doesn't.
    pub fn new(c: Cow<'a, Callable>) -> Result<Self, PreProcessingMiddlewareValidationError> {
        use PreProcessingMiddlewareValidationError::*;

        let mut output_type = c.output.as_ref().ok_or(CannotReturnTheUnitType)?.clone();

        // If it is fallible, we make sure that it returns a non-unit type on the happy path.
        if output_type.is_result() {
            let m = MatchResult::match_result(&output_type);
            output_type = m.ok.output;
            if output_type == ResolvedType::UNIT_TYPE {
                return Err(CannotFalliblyReturnTheUnitType);
            }
        }

        // We make sure that the callable doesn't have any unassigned generic type parameters.
        let mut free_parameters = IndexSet::new();
        for input in c.inputs.iter() {
            free_parameters.extend(input.unassigned_generic_type_parameters());
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

    pub fn into_owned(self) -> PreProcessingMiddleware<'static> {
        PreProcessingMiddleware {
            callable: Cow::Owned(self.callable.into_owned()),
        }
    }
}

#[derive(thiserror::Error, Debug, Clone)]
pub(crate) enum PreProcessingMiddlewareValidationError {
    #[error(
        "Pre-processing middlewares must return `pavex::middleware::Processing`.\n\
        This middleware doesn't: it returns the unit type, `()`."
    )]
    CannotReturnTheUnitType,
    #[error(
        "Pre-processing middlewares must return `pavex::middleware::Processing` when successful.\n\
        This middleware doesn't: it returns the unit type, `()`, when successful."
    )]
    CannotFalliblyReturnTheUnitType,
    #[error("Pre-processing middlewares can't have any *unassigned* generic type parameters")]
    UnderconstrainedGenericParameters { parameters: IndexSet<String> },
}
