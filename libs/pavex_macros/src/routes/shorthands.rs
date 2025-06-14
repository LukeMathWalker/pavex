use crate::fn_like::{Callable, CallableAnnotation, ImplContext};
use crate::utils::AnnotationCodegen;
use convert_case::{Case, Casing};
use quote::{format_ident, quote, quote_spanned};
use syn::Ident;

#[derive(darling::FromMeta, Debug, Clone)]
/// The available options for route macros.
pub struct InputSchema {
    pub path: String,
    pub id: Option<Ident>,
    pub error_handler: Option<String>,
}

impl TryFrom<InputSchema> for Properties {
    type Error = darling::Error;

    fn try_from(input: InputSchema) -> Result<Self, Self::Error> {
        let InputSchema {
            path,
            error_handler,
            id,
        } = input;
        Ok(Properties {
            path,
            id,
            error_handler,
        })
    }
}

#[derive(darling::FromMeta, Debug, Clone, PartialEq, Eq)]
struct Properties {
    pub path: String,
    pub error_handler: Option<String>,
    pub id: Option<Ident>,
}

#[derive(Clone, Copy)]
enum Method {
    Get,
    Delete,
    Head,
    Options,
    Patch,
    Post,
    Put,
}

impl Method {
    pub fn name(&self) -> &'static str {
        use Method::*;
        match self {
            Get => "GET",
            Delete => "DELETE",
            Head => "HEAD",
            Options => "OPTIONS",
            Patch => "PATCH",
            Post => "POST",
            Put => "PUT",
        }
    }
}

macro_rules! method_annotation {
    ($method:ident) => {
        paste::paste! {
            method_annotation!($method, [<$method:lower>]);
        }
    };
    ($method:ident,$lowercased:ident) => {
        paste::paste! {
            pub struct [<$method Annotation>];

            impl CallableAnnotation for [<$method Annotation>] {
                const PLURAL_COMPONENT_NAME: &str = "Request handlers";

                const ATTRIBUTE: &str = concat!("#[pavex::", stringify!($lowercased), "]");

                type InputSchema = InputSchema;

                fn codegen(
                    _impl_: Option<ImplContext>,
                    metadata: Self::InputSchema,
                    item: Callable,
                ) -> Result<AnnotationCodegen, proc_macro::TokenStream> {
                    method_shorthand(item.sig.ident, Method::$method, metadata)
                }
            }
        }
    };
}

method_annotation!(Get);
method_annotation!(Post);
method_annotation!(Patch);
method_annotation!(Put);
method_annotation!(Delete);
method_annotation!(Head);
method_annotation!(Options);

fn method_shorthand(
    name: Ident,
    method: Method,
    schema: InputSchema,
) -> Result<AnnotationCodegen, proc_macro::TokenStream> {
    let properties = schema
        .try_into()
        .map_err(|e: darling::Error| e.write_errors())?;
    Ok(emit(name, method, properties))
}

/// Decorate the input with a `#[diagnostic::pavex::route]` attribute
/// that matches the provided properties.
fn emit(name: Ident, method: Method, properties: Properties) -> AnnotationCodegen {
    let Properties {
        path,
        error_handler,
        id,
    } = properties;

    // Use the span of the function name if no identifier is provided.
    let id_span = id.as_ref().map(|id| id.span()).unwrap_or(name.span());
    // If the user didn't specify an identifier, generate one based on the function name.
    let name = name.to_string();
    let id = id.unwrap_or_else(|| format_ident!("{}", name.to_case(Case::Constant)));
    let id_str = id.to_string();
    let method_name = method.name();

    let mut properties = quote! {
        id = #id_str,
        method = #method_name,
        path = #path,
    };

    if let Some(error_handler) = error_handler {
        properties.extend(quote! {
            error_handler = #error_handler,
        });
    }

    let id_docs = format!(
        r#"A strongly-typed id to add [`{name}`] as a route to your Pavex application.

# Example

```rust,ignore
use pavex::blueprint::Blueprint;
// [...]
// ^ Import `{id}` here

let mut bp = Blueprint::new();
// Add `{name}` as a route to your application.
bp.route({id});
```"#
    );
    let pavex = quote! { ::pavex };
    let id_def = quote_spanned! { id_span =>
        #[doc = #id_docs]
        #[allow(unused)]
        pub const #id: #pavex::blueprint::raw::RawRoute = #pavex::blueprint::raw::RawRoute {
            coordinates: #pavex::blueprint::reflection::AnnotationCoordinates {
                id: #id_str,
                created_at: #pavex::created_at!(),
                macro_name: "route",
            }
        };
    };

    AnnotationCodegen {
        id_def: Some(id_def),
        new_attributes: vec![syn::parse_quote! {
            #[diagnostic::pavex::route(#properties)]
        }],
    }
}
