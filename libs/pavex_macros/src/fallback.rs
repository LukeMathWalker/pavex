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
}

impl TryFrom<InputSchema> for Properties {
    type Error = darling::Error;

    fn try_from(input: InputSchema) -> Result<Self, Self::Error> {
        let InputSchema { id, pavex } = input;
        Ok(Properties { id, pavex })
    }
}

#[derive(darling::FromMeta, Debug, Clone, PartialEq, Eq)]
pub struct Properties {
    pub id: Option<syn::Ident>,
    pub pavex: Option<syn::Ident>,
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
    let Properties { id, pavex } = properties;
    let id = id.unwrap_or_else(|| default_id(impl_.as_ref(), &item));
    let id_str = id.to_string();

    let properties = quote! {
        id = #id_str,
    };

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
