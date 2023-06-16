use std::borrow::Cow;

use indexmap::IndexSet;

use crate::compiler::computation::MatchResult;
use crate::compiler::utils::is_result;
use crate::language::{Callable, ResolvedType};

/// A callable that handles incoming requests for one or more routes.
/// It must return a type that implements `pavex::response::IntoResponse`.
/// It can be fallible, as long as the `Ok` type implements `pavex::response::IntoResponse`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct RequestHandler<'a> {
    pub(crate) callable: Cow<'a, Callable>,
}

impl<'a> RequestHandler<'a> {
    pub fn new(c: Cow<'a, Callable>) -> Result<Self, RequestHandlerValidationError> {
        let mut output_type = c
            .output
            .as_ref()
            .ok_or(RequestHandlerValidationError::CannotReturnTheUnitType)?
            .clone();

        // If the constructor is fallible, we make sure that it returns a non-unit type on
        // the happy path.
        if is_result(&output_type) {
            let m = MatchResult::match_result(&output_type);
            output_type = m.ok.output;
            if output_type == ResolvedType::UNIT_TYPE {
                return Err(RequestHandlerValidationError::CannotFalliblyReturnTheUnitType);
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
        `pavex::response::Response`.\n\
        This request handler doesn't: it returns the unit type, `()`. I can't convert `()` into an HTTP response."
    )]
    CannotReturnTheUnitType,
    #[error(
    "All request handlers must return a type that can be converted into a \
        `pavex::response::Response`.\n\
        This request handler doesn't: it returns the unit type, `()`, when successful. I can't convert `()` into an HTTP response."
    )]
    CannotFalliblyReturnTheUnitType,
    #[error("Input parameters for a request handler can't have any *unassigned* generic type parameters that appear exclusively in its input parameters.")]
    UnderconstrainedGenericParameters { parameters: IndexSet<String> },
}
