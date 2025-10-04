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
/// Post-processing middlewares must take a `Response` as input parameter.
///
/// # Output type
///
/// If infallible, the output type must implement `pavex::IntoResponse`.
/// If fallible, the output type must be a `Result<T, E>` where `T` implements
/// `pavex::IntoResponse`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct PostProcessingMiddleware<'a> {
    pub(crate) callable: Cow<'a, Callable>,
}

impl<'a> PostProcessingMiddleware<'a> {
    /// Creates a new post-processing middleware from a callable, either owned or borrowed.
    ///
    /// This function validates that the callable satisfies all the constraints of
    /// a post-processing middleware. An error is returned if it doesn't.
    pub fn new(
        c: Cow<'a, Callable>,
        response_type: &ResolvedType,
    ) -> Result<Self, PostProcessingMiddlewareValidationError> {
        use PostProcessingMiddlewareValidationError::*;

        let mut output_type = c.output.as_ref().ok_or(CannotReturnTheUnitType)?.clone();

        // If it is fallible, we make sure that it returns a non-unit type on the happy path.
        if output_type.is_result() {
            let m = MatchResult::match_result(&output_type);
            output_type = m.ok.output;
            if output_type == ResolvedType::UNIT_TYPE {
                return Err(CannotFalliblyReturnTheUnitType);
            }
        }

        // We verify that exactly one of the input parameters is a `Response`.
        {
            let response_parameters: Vec<_> =
                c.inputs.iter().filter(|t| *t == response_type).collect();
            if response_parameters.is_empty() {
                return Err(MustTakeResponseAsInputParameter);
            }
            if response_parameters.len() > 1 {
                return Err(CannotTakeMoreThanOneResponseAsInputParameter);
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

    /// Returns the index of the input parameter that is a `Response`.
    pub fn response_input_index(&self, response_type: &ResolvedType) -> usize {
        self.callable
            .inputs
            .iter()
            .position(|t| t == response_type)
            .unwrap()
    }

    pub fn into_owned(self) -> PostProcessingMiddleware<'static> {
        PostProcessingMiddleware {
            callable: Cow::Owned(self.callable.into_owned()),
        }
    }
}

#[derive(thiserror::Error, Debug, Clone)]
pub(crate) enum PostProcessingMiddlewareValidationError {
    #[error(
        "Post-processing middlewares must return a type that can be converted into a \
        `pavex::Response`.\n\
        This middleware doesn't: it returns the unit type, `()`. I can't convert `()` into an HTTP response."
    )]
    CannotReturnTheUnitType,
    #[error(
        "Post-processing middlewares must return a type that can be converted into a \
        `pavex::Response`.\n\
        This middleware doesn't: it returns the unit type, `()`, when successful. I can't convert `()` into an HTTP response."
    )]
    CannotFalliblyReturnTheUnitType,
    #[error(
        "Post-processing middlewares must take an instance of `pavex::Response` as one of their input parameters.\n\
        This middleware doesn't."
    )]
    MustTakeResponseAsInputParameter,
    #[error(
        "Post-processing middlewares can't take more than one instance of `pavex::Response` as input parameter.\n\
        This middleware does."
    )]
    CannotTakeMoreThanOneResponseAsInputParameter,
    #[error("Post-processing middlewares can't have any *unassigned* generic type parameters")]
    UnderconstrainedGenericParameters { parameters: IndexSet<String> },
}
