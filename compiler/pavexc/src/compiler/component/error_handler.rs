use std::fmt::{Display, Formatter};

use crate::compiler::component::CannotTakeMutReferenceError;
use crate::compiler::utils::get_err_variant;
use ahash::HashMap;
use indexmap::IndexSet;
use itertools::Itertools;

use crate::language::{Callable, FQPath, Lifetime, ResolvedType};

/// A transformation that, given a reference to an error type (and, optionally, other inputs),
/// returns an HTTP response.
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub(crate) struct ErrorHandler {
    pub(crate) callable: Callable,
    /// The index of the error type in the vector of input types for `callable`.
    pub(crate) error_ref_input_index: usize,
}

impl ErrorHandler {
    /// Validate an error handler.
    pub fn new(
        error_handler: Callable,
        error_ref_input_index: usize,
    ) -> Result<Self, ErrorHandlerValidationError> {
        Self::check_output_type(&error_handler)?;
        CannotTakeMutReferenceError::check_callable(&error_handler)?;
        Self::check_generic_params(
            &error_handler,
            error_ref_input_index,
            &error_handler.inputs[error_ref_input_index],
        )?;
        Ok(Self {
            callable: error_handler,
            error_ref_input_index,
        })
    }

    /// Validate an error handler for a specific fallible callable.
    pub fn for_fallible(
        error_handler: Callable,
        fallible_callable: &Callable,
        pavex_error: &ResolvedType,
    ) -> Result<Self, ErrorHandlerValidationError> {
        Self::check_output_type(&error_handler)?;

        let error_type = get_err_variant(fallible_callable.output.as_ref().unwrap());
        let (error_ref_input_index, error_ref) = error_handler
            .inputs
            .iter()
            .find_position(|t| {
                if let ResolvedType::Reference(t) = t {
                    !t.is_mutable
                        && (t.lifetime != Lifetime::Static)
                        && (t.inner.as_ref() == error_type || t.inner.as_ref() == pavex_error)
                } else {
                    // TODO: return a more specific error if the error handler takes the error as an input
                    //  parameter by value instead of taking it by reference.
                    false
                }
            })
            .ok_or_else(
                || ErrorHandlerValidationError::DoesNotTakeErrorReferenceAsInput {
                    fallible_callable: fallible_callable.to_owned(),
                    error_type: error_type.to_owned(),
                },
            )?;

        CannotTakeMutReferenceError::check_callable(&error_handler)?;
        Self::check_generic_params(&error_handler, error_ref_input_index, error_ref)?;

        Ok(Self {
            callable: error_handler,
            error_ref_input_index,
        })
    }

    /// Return the error type that this error handler takes as input.
    ///
    /// This is a **reference** to the error type returned by the fallible callable
    /// that this is error handler is associated with.
    pub(crate) fn error_type_ref(&self) -> &ResolvedType {
        &self.callable.inputs[self.error_ref_input_index]
    }

    /// Replace all unassigned generic type parameters in this error handler with the
    /// concrete types specified in `bindings`.
    ///
    /// The newly "bound" handler will be returned.
    pub fn bind_generic_type_parameters(&self, bindings: &HashMap<String, ResolvedType>) -> Self {
        Self {
            callable: self.callable.bind_generic_type_parameters(bindings),
            error_ref_input_index: self.error_ref_input_index,
        }
    }

    fn check_output_type(h: &Callable) -> Result<(), ErrorHandlerValidationError> {
        use ErrorHandlerValidationError::*;
        match &h.output {
            None => Err(CannotReturnTheUnitType(h.path.clone())),
            Some(output_type) => {
                if output_type.is_result() {
                    Err(CannotBeFallible(h.path.clone()))
                } else {
                    Ok(())
                }
            }
        }
    }

    fn check_generic_params(
        h: &Callable,
        error_ref_index: usize,
        error_ref: &ResolvedType,
    ) -> Result<(), ErrorHandlerValidationError> {
        use ErrorHandlerValidationError::*;

        // All "free" generic parameters in the error handler must be assigned to concrete types.
        // The only ones that are allowed to be unassigned are those used by the error type,
        // because they might/will be dictated by the fallible callable that this error handler
        // is associated with.
        let error_ref_unassigned_generic_parameters =
            error_ref.unassigned_generic_type_parameters();
        let mut free_parameters = IndexSet::new();
        for (i, input) in h.inputs.iter().enumerate() {
            if i == error_ref_index {
                continue;
            }
            free_parameters.extend(
                input
                    .unassigned_generic_type_parameters()
                    .difference(&error_ref_unassigned_generic_parameters)
                    .cloned(),
            );
        }
        if !free_parameters.is_empty() {
            Err(UnderconstrainedGenericParameters {
                parameters: free_parameters,
                error_ref_input_index: error_ref_index,
            })
        } else {
            Ok(())
        }
    }
}

impl From<ErrorHandler> for Callable {
    fn from(e: ErrorHandler) -> Self {
        e.callable
    }
}

impl AsRef<Callable> for ErrorHandler {
    fn as_ref(&self) -> &Callable {
        &self.callable
    }
}

#[derive(thiserror::Error, Debug, Clone)]
pub(crate) enum ErrorHandlerValidationError {
    CannotReturnTheUnitType(FQPath),
    CannotBeFallible(FQPath),
    CannotTakeAMutableReferenceAsInput(#[from] CannotTakeMutReferenceError),
    DoesNotTakeErrorReferenceAsInput {
        fallible_callable: Callable,
        error_type: ResolvedType,
    },
    UnderconstrainedGenericParameters {
        parameters: IndexSet<String>,
        error_ref_input_index: usize,
    },
}

impl Display for ErrorHandlerValidationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorHandlerValidationError::CannotReturnTheUnitType(path) => {
                write!(
                    f,
                    "All error handlers must return a type that implements `pavex::IntoResponse`.\n`{path}` doesn't, it returns the unit type, `()`. I can't convert `()` into an HTTP response!"
                )
            }
            ErrorHandlerValidationError::DoesNotTakeErrorReferenceAsInput {
                fallible_callable,
                error_type,
                ..
            } => {
                write!(
                    f,
                    "Error handlers associated with a fallible operation must take a reference \
                    to the operation's error type as input.\n\
                    This error handler is associated with `{}`, therefore I \
                    expect `&{error_type:?}` to be one of its input parameters.",
                    fallible_callable.path,
                )
            }
            ErrorHandlerValidationError::UnderconstrainedGenericParameters { .. } => {
                write!(
                    f,
                    "Input parameters for an error handler can't have any *unassigned* \
                       generic type parameters that do not appear in the error type itself."
                )
            }
            ErrorHandlerValidationError::CannotBeFallible(path) => {
                write!(
                    f,
                    "Error handlers must be infallible.\n`{path}` isn't, it returns a `Result`!"
                )
            }
            ErrorHandlerValidationError::CannotTakeAMutableReferenceAsInput(e) => {
                write!(f, "{e}")
            }
        }
    }
}
