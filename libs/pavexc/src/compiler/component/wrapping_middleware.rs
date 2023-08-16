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
/// Middlewares must take a `Next<_>` as input parameter—it encapsulates the rest of the processing
/// of the request (i.e. the next middlewares and, eventually, the request handler).
///
/// # Output type
///
/// If infallible, the output type must implement `pavex::response::IntoResponse`.
/// If fallible, the output type must be a `Result<T, E>` where `T` implements
/// `pavex::response::IntoResponse`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct WrappingMiddleware<'a> {
    pub(crate) callable: Cow<'a, Callable>,
}

impl<'a> WrappingMiddleware<'a> {
    /// Creates a new wrapping middleware from a callable, either owned or borrowed.
    ///
    /// This function validates that the callable satisfies all the constraints of
    /// a wrapping middleware. An error is returned if it doesn't.
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

        // We verify that exactly one of the input parameters is a `Next<_>`.
        let next_unassigned_generic_parameters = {
            let next_parameters: Vec<_> = c.inputs.iter().filter(|t| is_next(*t)).collect();
            if next_parameters.is_empty() {
                return Err(MustTakeNextAsInputParameter);
            }
            if next_parameters.len() > 1 {
                return Err(CannotTakeMoreThanOneNextAsInputParameter);
            }
            next_parameters[0].unassigned_generic_type_parameters()
        };

        // We make sure that the callable doesn't have any unassigned generic type parameters
        // apart from the one used in Next.
        let allowed_unassigned_generic_parameters = next_unassigned_generic_parameters;
        let mut free_parameters = IndexSet::new();
        for input in c.inputs.iter() {
            free_parameters.extend(
                input
                    .unassigned_generic_type_parameters()
                    .difference(&allowed_unassigned_generic_parameters)
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

    /// Returns `true` if this middleware is fallible—that is, if it returns a `Result`.
    pub fn is_fallible(&self) -> bool {
        self.output_type().is_result()
    }

    /// Returns the index of the input parameter that is a `Next<_>`.
    pub fn next_input_index(&self) -> usize {
        self.callable.inputs.iter().position(is_next).unwrap()
    }

    pub fn into_owned(self) -> WrappingMiddleware<'static> {
        WrappingMiddleware {
            callable: Cow::Owned(self.callable.into_owned()),
        }
    }
}

/// Returns `true` if the given type is an owned `Next<_>`.
fn is_next(t: &ResolvedType) -> bool {
    let ResolvedType::ResolvedPath(t) = t else {
            return false;
        };
    t.base_type == ["pavex", "middleware", "Next"]
}

#[derive(thiserror::Error, Debug, Clone)]
pub(crate) enum WrappingMiddlewareValidationError {
    #[error(
        "Wrapping middlewares must return a type that can be converted into a \
        `pavex::response::Response`.\n\
        This middleware doesn't: it returns the unit type, `()`. I can't convert `()` into an HTTP response."
    )]
    CannotReturnTheUnitType,
    #[error(
        "Wrapping middlewares must return a type that can be converted into a \
        `pavex::response::Response`.\n\
        This middleware doesn't: it returns the unit type, `()`, when successful. I can't convert `()` into an HTTP response."
    )]
    CannotFalliblyReturnTheUnitType,
    #[error(
        "Wrapping middlewares must take an instance of `pavex::middleware::Next<_>` as input parameter.\n\
        This middleware doesn't."
    )]
    MustTakeNextAsInputParameter,
    #[error(
        "Wrapping middlewares can't take more than one instance of `pavex::middleware::Next<_>` as input parameter.\n\
        This middleware does."
    )]
    CannotTakeMoreThanOneNextAsInputParameter,
    #[error(
        "Wrapping middlewares can't have any *unassigned* generic type parameters \
        apart from the one used in `pavex::middleware::Next<_>`."
    )]
    UnderconstrainedGenericParameters { parameters: IndexSet<String> },
}
