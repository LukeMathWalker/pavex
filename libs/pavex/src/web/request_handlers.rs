use std::borrow::Cow;

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
        Ok(Self { callable: c })
    }

    pub fn output_type(&self) -> &ResolvedType {
        self.callable.output.as_ref().unwrap()
    }

    pub fn input_types(&self) -> &[ResolvedType] {
        self.callable.inputs.as_slice()
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
}
