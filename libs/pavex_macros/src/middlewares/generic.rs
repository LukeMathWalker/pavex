use convert_case::{Case, Casing};
use quote::{format_ident, quote, quote_spanned};
use syn::Ident;

use crate::{fn_like::Callable, utils::AnnotationCodegen};

#[derive(darling::FromMeta, Debug, Clone)]
/// The available options for middleware macros.
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

#[derive(Clone, Copy)]
pub enum MiddlewareKind {
    Wrap,
    PreProcess,
    PostProcess,
}

impl MiddlewareKind {
    pub fn macro_name(&self) -> &'static str {
        match self {
            MiddlewareKind::Wrap => "wrap",
            MiddlewareKind::PreProcess => "pre_process",
            MiddlewareKind::PostProcess => "post_process",
        }
    }

    pub fn raw_type_name(&self) -> syn::Ident {
        let s = match self {
            MiddlewareKind::Wrap => "RawWrappingMiddleware",
            MiddlewareKind::PreProcess => "RawPreProcessingMiddleware",
            MiddlewareKind::PostProcess => "RawPostProcessingMiddleware",
        };
        format_ident!("{s}")
    }
}

pub fn middleware(
    kind: MiddlewareKind,
    schema: InputSchema,
    item: Callable,
) -> Result<AnnotationCodegen, proc_macro::TokenStream> {
    let properties = schema
        .try_into()
        .map_err(|e: darling::Error| e.write_errors())?;
    Ok(emit(kind, item.sig.ident, properties))
}

/// Decorate the input with a diagnostic attribute
/// that matches the provided properties.
fn emit(kind: MiddlewareKind, name: Ident, properties: Properties) -> AnnotationCodegen {
    let Properties { id, pavex } = properties;
    // Use the span of the function name if no identifier is provided.
    let id_span = id.as_ref().map(|id| id.span()).unwrap_or(name.span());

    let name = name.to_string();

    // If the user didn't specify an identifier, generate one based on the function name.
    let id = id.unwrap_or_else(|| format_ident!("{}", name.to_case(Case::Constant)));
    let properties = quote! {
        id = #id,
    };

    let id_docs = {
        let adj = match kind {
            MiddlewareKind::Wrap => "wrapping",
            MiddlewareKind::PreProcess => "pre-processing",
            MiddlewareKind::PostProcess => "post-processing",
        };
        let bp_method_name = match kind {
            MiddlewareKind::Wrap => "wrap",
            MiddlewareKind::PreProcess => "pre_process",
            MiddlewareKind::PostProcess => "post_process",
        };
        format!(
            r#"A strongly-typed id to add [`{name}`] as a {adj} middleware to your Pavex application.

# Example

```rust,ignore
use pavex::blueprint::Blueprint;
// [...]
// ^ Import `{id}` here

let mut bp = Blueprint::new();
// Add `{name}` as a {adj} middleware to your application.
bp.{bp_method_name}({id});
```"#
        )
    };
    let macro_name = kind.macro_name();
    let raw_type_name = kind.raw_type_name();
    let pavex = match pavex {
        Some(c) => quote! { #c },
        None => quote! { ::pavex },
    };
    let id_def = quote_spanned! { id_span =>
        #[doc = #id_docs]
        pub const #id: #pavex::blueprint::raw::#raw_type_name = #pavex::blueprint::raw::#raw_type_name {
            coordinates: #pavex::with_location!(#pavex::blueprint::reflection::RawIdentifiers {
                import_path: concat!(module_path!(), "::", #name),
                macro_name: #macro_name,
            }),
        };
    };

    let macro_name = format_ident!("{macro_name}");
    AnnotationCodegen {
        id_def: Some(id_def),
        new_attributes: vec![syn::parse_quote! { #[diagnostic::pavex::#macro_name(#properties)] }],
    }
}
