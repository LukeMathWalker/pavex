use convert_case::{Case, Casing};
use quote::{format_ident, quote, quote_spanned};
use syn::Ident;

use crate::{
    utils::AnnotationCodegen,
    utils::fn_like::{Callable, CallableAnnotation, ImplContext},
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
        _impl_: Option<ImplContext>,
        metadata: Self::InputSchema,
        item: Callable,
    ) -> Result<AnnotationCodegen, proc_macro::TokenStream> {
        let properties = metadata
            .try_into()
            .map_err(|e: darling::Error| e.write_errors())?;
        Ok(emit(item.sig.ident, properties))
    }
}

/// Decorate the input with a `#[diagnostic::pavex::fallback]` attribute
/// that matches the provided properties.
fn emit(name: Ident, properties: Properties) -> AnnotationCodegen {
    let Properties { id, pavex } = properties;
    let pavex = match pavex {
        Some(c) => quote! { #c },
        None => quote! { ::pavex },
    };
    // Use the span of the function name if no identifier is provided.
    let id_span = id.as_ref().map(|id| id.span()).unwrap_or(name.span());

    let name = name.to_string();

    // If the user didn't specify an identifier, generate one based on the function name.
    let id = id.unwrap_or_else(|| format_ident!("{}", name.to_case(Case::Constant)));
    let id_str = id.to_string();
    let properties = quote! {
        id = #id_str,
    };

    let id_docs = {
        format!(
            r#"A strongly-typed id to add [`{name}`] as a fallback handler to your Pavex application.

# Example

```rust,ignore
use pavex::blueprint::Blueprint;
// [...]
// ^ Import `{id}` here

let mut bp = Blueprint::new();
// Add `{name}` as a fallback handler to your application.
bp.fallback({id});
```"#
        )
    };
    let id_def = quote_spanned! { id_span =>
        #[doc = #id_docs]
        pub const #id: #pavex::blueprint::raw::RawFallback = #pavex::blueprint::raw::RawFallback {
            coordinates: #pavex::blueprint::reflection::AnnotationCoordinates {
                id: #id_str,
                created_at: #pavex::created_at!(),
                macro_name: "fallback",
            },
        };
    };

    AnnotationCodegen {
        id_def: Some(id_def),
        new_attributes: vec![syn::parse_quote! { #[diagnostic::pavex::fallback(#properties)] }],
    }
}
