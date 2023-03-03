use std::borrow::Cow;

use indexmap::IndexSet;

use crate::language::{Callable, ResolvedType};

/// A callable that handles incoming requests for one or more routes.
/// It must return a type that implements `pavex_runtime::response::IntoResponse`.
/// It can be fallible, as long as the `Ok` type implements `pavex_runtime::response::IntoResponse`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct RequestHandler<'a> {
    pub(crate) callable: Cow<'a, Callable>,
}

impl<'a> RequestHandler<'a> {
    pub fn new(c: Cow<'a, Callable>) -> Result<Self, RequestHandlerValidationError> {
        if c.output.is_none() {
            return Err(RequestHandlerValidationError::CannotReturnTheUnitType);
        }
        let output_type = c.output.as_ref().unwrap();

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
            return Err(
                RequestHandlerValidationError::UnderconstrainedGenericParameters {
                    parameters: free_parameters,
                },
            );
        }

        Ok(Self { callable: c })
    }

    pub fn output_type(&self) -> &ResolvedType {
        self.callable.output.as_ref().unwrap()
    }

    pub fn input_types(&self) -> &[ResolvedType] {
        self.callable.inputs.as_slice()
    }

    pub fn into_owned(self) -> RequestHandler<'static> {
        RequestHandler {
            callable: Cow::Owned(self.callable.into_owned()),
        }
    }
}

#[derive(thiserror::Error, Debug, Clone)]
pub(crate) enum RequestHandlerValidationError {
    #[error(
        "All request handlers must return a type that can be converted into a \
        `pavex_runtime::response::Response`.\n\
        This request handler doesn't: it returns the unit type, `()`."
    )]
    CannotReturnTheUnitType,
    #[error("Input parameters for a request handler cannot have any *unassigned* generic type parameters that appear exclusively in its input parameters.")]
    UnderconstrainedGenericParameters { parameters: IndexSet<String> },
}
