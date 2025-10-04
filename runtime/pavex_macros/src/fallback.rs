use quote::quote;

use crate::utils::{
    AnnotationCodegen,
    fn_like::{Callable, CallableAnnotation, ImplContext},
    id::{callable_id_def, default_id},
};

#[derive(darling::FromMeta, Debug, Clone)]
/// The available options for fallbacks.
pub struct InputSchema {
    pub id: Option<syn::Ident>,
    pub pavex: Option<syn::Ident>,
    pub allow: Option<FallbackAllows>,
}

#[derive(darling::FromMeta, Debug, Clone)]
pub struct FallbackAllows {
    error_fallback: darling::util::Flag,
}

impl TryFrom<InputSchema> for Properties {
    type Error = darling::Error;

    fn try_from(input: InputSchema) -> Result<Self, Self::Error> {
        let InputSchema { id, pavex, allow } = input;
        let allow_error_fallback = allow.as_ref().map(|a| a.error_fallback.is_present());
        Ok(Properties {
            id,
            pavex,
            allow_error_fallback,
        })
    }
}

#[derive(darling::FromMeta, Debug, Clone, PartialEq, Eq)]
pub struct Properties {
    pub id: Option<syn::Ident>,
    pub pavex: Option<syn::Ident>,
    pub allow_error_fallback: Option<bool>,
}

pub struct FallbackAnnotation;

impl CallableAnnotation for FallbackAnnotation {
    const PLURAL_COMPONENT_NAME: &str = "Fallbacks";

    const ATTRIBUTE: &str = "#[pavex::fallback]";

    type InputSchema = InputSchema;

    fn codegen(
        impl_: Option<ImplContext>,
        metadata: Self::InputSchema,
        item: Callable,
    ) -> Result<AnnotationCodegen, proc_macro::TokenStream> {
        let properties = metadata
            .try_into()
            .map_err(|e: darling::Error| e.write_errors())?;
        Ok(emit(impl_, item, properties))
    }
}

/// Decorate the input with a `#[diagnostic::pavex::fallback]` attribute
/// that matches the provided properties.
fn emit(impl_: Option<ImplContext>, item: Callable, properties: Properties) -> AnnotationCodegen {
    let Properties {
        id,
        pavex,
        allow_error_fallback,
    } = properties;
    let id = id.unwrap_or_else(|| default_id(impl_.as_ref(), &item));
    let id_str = id.to_string();

    let mut properties = quote! {
        id = #id_str,
    };

    if let Some(allow_error_fallback) = allow_error_fallback {
        properties.extend(quote! {
            allow_error_fallback = #allow_error_fallback,
        });
    }

    AnnotationCodegen {
        id_def: Some(callable_id_def(
            &id,
            pavex.as_ref(),
            "fallback",
            "Fallback",
            "a fallback handler",
            "fallback",
            false,
            impl_.as_ref(),
            &item,
        )),
        new_attributes: vec![syn::parse_quote! { #[diagnostic::pavex::fallback(#properties)] }],
    }
}
