use crate::language::{Callable, ResolvedPath, ResolvedType};
use crate::web::utils::is_result;

/// A transformation that, given a reference to an error type (and, optionally, other inputs),
/// returns an HTTP response.
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub(crate) struct ErrorHandler {
    pub(crate) callable: Callable,
    /// The index of the error type in the vector of input types for `callable`.
    pub(crate) error_input_index: usize,
    pub(crate) fallible_callable: Callable,
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
            let mut e = result_type.generic_arguments[1].clone();
            e.is_shared_reference = true;
            e
        };
        let error_input_index = error_handler
            .inputs
            .iter()
            .position(|i| i == &error_type_ref);
        match error_input_index {
            Some(i) => Ok(Self {
                callable: error_handler,
                error_input_index: i,
                fallible_callable: fallible_callable.to_owned(),
            }),
            None => Err(
                ErrorHandlerValidationError::DoesNotTakeErrorReferenceAsInput {
                    error_handler,
                    fallible_callable: fallible_callable.to_owned(),
                },
            ),
        }
    }

    /// Return the error type that this error handler takes as input.
    ///
    /// This is a **reference** to the error type returned by the fallible callable
    /// that this is error handler is associated with.
    pub(crate) fn error_type(&self) -> &ResolvedType {
        &self.callable.inputs[self.error_input_index]
    }

    pub fn output_type(&self) -> &ResolvedType {
        self.callable.output.as_ref().unwrap()
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

#[derive(thiserror::Error, Debug)]
pub(crate) enum ErrorHandlerValidationError {
    #[error("I expect all error handlers to return *something*.\nThis doesn't: it returns the unit type, `()`.")]
    CannotReturnTheUnitType(ResolvedPath),
    #[error("I expect the error handler associated with a fallible operation to take a reference to the operation's error type as input.")]
    DoesNotTakeErrorReferenceAsInput {
        error_handler: Callable,
        fallible_callable: Callable,
    },
}
