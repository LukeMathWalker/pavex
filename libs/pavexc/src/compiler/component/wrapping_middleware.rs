use crate::language::{Callable, ResolvedType};
use std::borrow::Cow;

/// A callable that gets invoked during the processing of incoming requests
/// for one or more routes.
/// It must return a type that implements `pavex::response::IntoResponse`.
/// It can be fallible, as long as the `Ok` type implements `pavex::response::IntoResponse`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct WrappingMiddleware<'a> {
    pub(crate) callable: Cow<'a, Callable>,
}

impl<'a> WrappingMiddleware<'a> {
    pub fn output_type(&self) -> &ResolvedType {
        self.callable.output.as_ref().unwrap()
    }

    pub fn input_types(&self) -> &[ResolvedType] {
        self.callable.inputs.as_slice()
    }

    pub fn into_owned(self) -> WrappingMiddleware<'static> {
        WrappingMiddleware {
            callable: Cow::Owned(self.callable.into_owned()),
        }
    }
}