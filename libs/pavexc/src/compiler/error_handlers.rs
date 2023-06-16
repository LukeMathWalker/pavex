use std::fmt::{Display, Formatter};

use ahash::HashMap;
use indexmap::IndexSet;

use crate::compiler::utils::is_result;
use crate::language::{Callable, GenericArgument, ResolvedPath, ResolvedType, TypeReference};

/// A transformation that, given a reference to an error type (and, optionally, other inputs),
/// returns an HTTP response.
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub(crate) struct ErrorHandler {
    pub(crate) callable: Callable,
    /// The index of the error type in the vector of input types for `callable`.
    pub(crate) error_input_index: usize,
}

impl ErrorHandler {
    pub fn new(
        error_handler: Callable,
        fallible_callable: &Callable,
    ) -> Result<Self, ErrorHandlerValidationError> {
        if error_handler.output.is_none() {
            return Err(ErrorHandlerValidationError::CannotReturnTheUnitType(
                error_handler.path,
            ));
        }
        let result_type = fallible_callable
            .output
            .as_ref()
            .expect("Fallible callable must have an output type")
            .clone();
        assert!(
            is_result(&result_type),
            "Fallible callable must return a Result"
        );
        let error_type_ref = {
            let ResolvedType::ResolvedPath(result_type) = result_type else {
                unreachable!()
            };
            let GenericArgument::TypeParameter(e) = result_type.generic_arguments[1].clone() else {
                unreachable!()
            };
            ResolvedType::Reference(TypeReference {
                is_mutable: false,
                is_static: false,
                inner: Box::new(e),
            })
        };
        // TODO: verify that the error handler does NOT return a `Result`
        // TODO: verify that the error handler returns a type that implements `IntoResponse`
        // TODO: return a more specific error if the error handler takes the error as an input
        //  parameter by value instead of taking it by reference.
        let error_input_index = error_handler
            .inputs
            .iter()
            .position(|i| i == &error_type_ref)
            .ok_or_else(
                || ErrorHandlerValidationError::DoesNotTakeErrorReferenceAsInput {
                    fallible_callable: fallible_callable.to_owned(),
                    error_type: error_type_ref,
                },
            )?;
        let error_ref_parameter = error_handler
            .inputs
            .get(error_input_index)
            .expect("Error input index must be valid");

        // All "free" generic parameters in the error handler must be assigned to concrete types.
        // The only ones that are allowed to be unassigned are those used by the error type,
        // because they might/will be dictated by the fallible callable that this error handler
        // is associated with.
        let error_ref_unassigned_generic_parameters =
            error_ref_parameter.unassigned_generic_type_parameters();
        let mut free_parameters = IndexSet::new();
        for (i, input) in error_handler.inputs.iter().enumerate() {
            if i == error_input_index {
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
            return Err(
                ErrorHandlerValidationError::UnderconstrainedGenericParameters {
                    parameters: free_parameters,
                    error_ref_input_index: error_input_index,
                },
            );
        }

        Ok(Self {
            callable: error_handler,
            error_input_index,
        })
    }

    /// Return the error type that this error handler takes as input.
    ///
    /// This is a **reference** to the error type returned by the fallible callable
    /// that this is error handler is associated with.
    pub(crate) fn error_type_ref(&self) -> &ResolvedType {
        &self.callable.inputs[self.error_input_index]
    }

    pub fn output_type(&self) -> &ResolvedType {
        self.callable.output.as_ref().unwrap()
    }

    pub fn input_types(&self) -> &[ResolvedType] {
        self.callable.inputs.as_slice()
    }

    /// Replace all unassigned generic type parameters in this error handler with the
    /// concrete types specified in `bindings`.
    ///
    /// The newly "bound" error handler will be returned.
    pub fn bind_generic_type_parameters(&self, bindings: &HashMap<String, ResolvedType>) -> Self {
        Self {
            callable: self.callable.bind_generic_type_parameters(bindings),
            error_input_index: self.error_input_index,
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
    CannotReturnTheUnitType(ResolvedPath),
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
            ErrorHandlerValidationError::CannotReturnTheUnitType(_) => {
                write!(f, "All error handlers must return a type that implements `pavex::response::IntoResponse`.\nThis error handler doesn't: it returns the unit type, `()`. I can't convert `()` into an HTTP response!")
            }
            ErrorHandlerValidationError::DoesNotTakeErrorReferenceAsInput {
                ref fallible_callable,
                ref error_type,
                ..
            } => {
                write!(
                    f,
                    "Error handlers associated with a fallible operation must take a reference \
                    to the operation's error type as input.\n\
                    This error handler is associated with `{}`, therefore I \
                    expect `{error_type:?}` to be one of its input parameters.",
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
        }
    }
}
