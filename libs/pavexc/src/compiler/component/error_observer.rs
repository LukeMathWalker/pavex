use std::borrow::Cow;
use std::fmt::{Display, Formatter};

use indexmap::IndexSet;

use crate::language::{Callable, ResolvedPath, ResolvedType};

/// A computation applied to an unhandled error that has been converted into Pavex's "common"
/// error type.
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub(crate) struct ErrorObserver<'a> {
    pub(crate) callable: Cow<'a, Callable>,
    /// The index of the error type in the vector of input types for `callable`.
    pub(crate) error_input_index: usize,
}

impl<'a> ErrorObserver<'a> {
    pub fn new(
        error_observer: Cow<'a, Callable>,
        pavex_error_ref: &ResolvedType,
    ) -> Result<Self, ErrorObserverValidationError> {
        if let Some(output_type) = &error_observer.output {
            return Err(ErrorObserverValidationError::MustReturnUnitType {
                observer_path: error_observer.path.to_owned(),
                output_type: output_type.to_owned(),
            });
        }
        // TODO: return a more specific error if the error observer takes the error input
        //  parameter by value instead of taking it by reference.
        let error_input_index = error_observer
            .inputs
            .iter()
            .position(|i| i == pavex_error_ref)
            .ok_or_else(
                || ErrorObserverValidationError::DoesNotTakeErrorReferenceAsInput {
                    observer_path: error_observer.path.to_owned(),
                    error_type: pavex_error_ref.to_owned(),
                },
            )?;

        // All "free" generic parameters in the error handler must be assigned to concrete types.
        // The only ones that are allowed to be unassigned are those used by the error type,
        // because they might/will be dictated by the fallible callable that this error handler
        // is associated with.
        let mut free_parameters = IndexSet::new();
        for input in &error_observer.inputs {
            free_parameters.extend(input.unassigned_generic_type_parameters());
        }
        if !free_parameters.is_empty() {
            return Err(ErrorObserverValidationError::UnassignedGenericParameters {
                parameters: free_parameters,
                observer_path: error_observer.path.to_owned(),
            });
        }

        Ok(Self {
            callable: error_observer,
            error_input_index,
        })
    }

    /// Return the error type that this error observer takes as input.
    ///
    /// This is a **reference** to Pavex's common error type.
    pub(crate) fn error_type_ref(&self) -> &ResolvedType {
        &self.callable.inputs[self.error_input_index]
    }

    pub fn into_owned(self) -> ErrorObserver<'static> {
        ErrorObserver {
            callable: Cow::Owned(self.callable.into_owned()),
            error_input_index: self.error_input_index,
        }
    }

    pub fn input_types(&self) -> &[ResolvedType] {
        self.callable.inputs.as_slice()
    }
}

impl<'a> AsRef<Callable> for ErrorObserver<'a> {
    fn as_ref(&self) -> &Callable {
        self.callable.as_ref()
    }
}

#[derive(thiserror::Error, Debug, Clone)]
pub(crate) enum ErrorObserverValidationError {
    MustReturnUnitType {
        observer_path: ResolvedPath,
        output_type: ResolvedType,
    },
    DoesNotTakeErrorReferenceAsInput {
        observer_path: ResolvedPath,
        error_type: ResolvedType,
    },
    UnassignedGenericParameters {
        observer_path: ResolvedPath,
        parameters: IndexSet<String>,
    },
}

impl Display for ErrorObserverValidationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorObserverValidationError::MustReturnUnitType {
                output_type,
                observer_path,
            } => {
                write!(
                    f,
                    "Error observers must have no return type.\n`{observer_path}` returns `{output_type:?}`."
                )
            }
            ErrorObserverValidationError::DoesNotTakeErrorReferenceAsInput {
                observer_path,
                error_type,
            } => {
                write!(
                    f,
                    "Error observers must take a reference to Pavex's common error type as input (`{error_type:?}`).\n\
                    `{}` doesn't.",
                    observer_path,
                )
            }
            ErrorObserverValidationError::UnassignedGenericParameters {
                observer_path,
                parameters,
            } => {
                write!(
                    f,
                    "All generic parameters must be assigned to a concrete type when you register an error observer, I can't infer them.\n\
                    `{observer_path}` has {} unassigned generic parameters.",
                    parameters.len()
                )
            }
        }
    }
}
