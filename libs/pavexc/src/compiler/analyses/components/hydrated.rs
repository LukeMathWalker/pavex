use crate::compiler::analyses::components::component::TransformerInfo;
use crate::compiler::component::{
    Constructor, ErrorObserver, PostProcessingMiddleware, PreProcessingMiddleware, RequestHandler,
    WrappingMiddleware,
};
use crate::compiler::computation::Computation;
use crate::language::ResolvedType;
use std::borrow::Cow;

/// A transformation that, given a set of inputs, **constructs** a new type.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum HydratedComponent<'a> {
    Constructor(Constructor<'a>),
    RequestHandler(RequestHandler<'a>),
    WrappingMiddleware(WrappingMiddleware<'a>),
    PreProcessingMiddleware(PreProcessingMiddleware<'a>),
    PostProcessingMiddleware(PostProcessingMiddleware<'a>),
    Transformer(Computation<'a>, TransformerInfo),
    ErrorObserver(ErrorObserver<'a>),
}

impl<'a> HydratedComponent<'a> {
    pub(crate) fn input_types(&self) -> Cow<[ResolvedType]> {
        match self {
            HydratedComponent::Constructor(c) => c.input_types(),
            HydratedComponent::RequestHandler(r) => Cow::Borrowed(r.input_types()),
            HydratedComponent::Transformer(c, ..) => c.input_types(),
            HydratedComponent::WrappingMiddleware(c) => Cow::Borrowed(c.input_types()),
            HydratedComponent::PostProcessingMiddleware(p) => Cow::Borrowed(p.input_types()),
            HydratedComponent::PreProcessingMiddleware(p) => Cow::Borrowed(p.input_types()),
            HydratedComponent::ErrorObserver(eo) => Cow::Borrowed(eo.input_types()),
        }
    }

    pub(crate) fn output_type(&self) -> Option<&ResolvedType> {
        match self {
            HydratedComponent::Constructor(c) => Some(c.output_type()),
            HydratedComponent::RequestHandler(r) => Some(r.output_type()),
            HydratedComponent::WrappingMiddleware(e) => Some(e.output_type()),
            HydratedComponent::PostProcessingMiddleware(p) => Some(p.output_type()),
            HydratedComponent::PreProcessingMiddleware(p) => Some(p.output_type()),
            // TODO: we are not enforcing that the output type of a transformer is not
            //  the unit type. In particular, you can successfully register a `Result<T, ()>`
            //  type, which will result into a `MatchResult` with output `()` for the error.
            HydratedComponent::Transformer(c, ..) => c.output_type(),
            HydratedComponent::ErrorObserver(_) => None,
        }
    }

    pub(crate) fn is_fallible(&self) -> bool {
        self.output_type().map_or(false, |t| t.is_result())
    }

    /// Returns a [`Computation`] that matches the transformation carried out by this component.
    pub(crate) fn computation(&self) -> Computation<'a> {
        match self {
            HydratedComponent::Constructor(c) => c.0.clone(),
            HydratedComponent::RequestHandler(r) => r.callable.clone().into(),
            HydratedComponent::WrappingMiddleware(w) => w.callable.clone().into(),
            HydratedComponent::PostProcessingMiddleware(p) => p.callable.clone().into(),
            HydratedComponent::PreProcessingMiddleware(p) => p.callable.clone().into(),
            HydratedComponent::Transformer(t, ..) => t.clone(),
            HydratedComponent::ErrorObserver(eo) => eo.callable.clone().into(),
        }
    }

    pub(crate) fn into_owned(self) -> HydratedComponent<'static> {
        match self {
            HydratedComponent::Constructor(c) => HydratedComponent::Constructor(c.into_owned()),
            HydratedComponent::RequestHandler(r) => {
                HydratedComponent::RequestHandler(r.into_owned())
            }
            HydratedComponent::WrappingMiddleware(w) => {
                HydratedComponent::WrappingMiddleware(w.into_owned())
            }
            HydratedComponent::Transformer(t, i) => {
                HydratedComponent::Transformer(t.into_owned(), i)
            }
            HydratedComponent::ErrorObserver(eo) => {
                HydratedComponent::ErrorObserver(eo.into_owned())
            }
            HydratedComponent::PostProcessingMiddleware(p) => {
                HydratedComponent::PostProcessingMiddleware(p.into_owned())
            }
            HydratedComponent::PreProcessingMiddleware(p) => {
                HydratedComponent::PreProcessingMiddleware(p.into_owned())
            }
        }
    }
}
