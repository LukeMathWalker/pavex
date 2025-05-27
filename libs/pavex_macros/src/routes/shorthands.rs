use crate::fn_like::{Callable, CallableAnnotation, ImplContext};
use crate::utils::AnnotationCodegen;
use quote::quote;

#[derive(darling::FromMeta, Debug, Clone)]
/// The available options for route macros.
pub struct InputSchema {
    pub path: String,
    pub error_handler: Option<String>,
}

impl TryFrom<InputSchema> for Properties {
    type Error = darling::Error;

    fn try_from(input: InputSchema) -> Result<Self, Self::Error> {
        let InputSchema {
            path,
            error_handler,
        } = input;
        Ok(Properties {
            path,
            error_handler,
        })
    }
}

#[derive(darling::FromMeta, Debug, Clone, PartialEq, Eq)]
struct Properties {
    pub path: String,
    pub error_handler: Option<String>,
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
                    _item: Callable,
                ) -> Result<AnnotationCodegen, proc_macro::TokenStream> {
                    method_shorthand(Method::$method, metadata)
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
    method: Method,
    schema: InputSchema,
) -> Result<AnnotationCodegen, proc_macro::TokenStream> {
    let properties = schema
        .try_into()
        .map_err(|e: darling::Error| e.write_errors())?;
    Ok(emit(method, properties))
}

/// Decorate the input with a `#[diagnostic::pavex::route]` attribute
/// that matches the provided properties.
fn emit(method: Method, properties: Properties) -> AnnotationCodegen {
    let Properties {
        path,
        error_handler,
    } = properties;

    let method_name = method.name();
    let mut properties = quote! {
        method = #method_name,
        path = #path,
    };

    if let Some(error_handler) = error_handler {
        properties.extend(quote! {
            error_handler = #error_handler,
        });
    }

    AnnotationCodegen {
        id_def: None,
        new_attributes: vec![syn::parse_quote! {
            #[diagnostic::pavex::route(#properties)]
        }],
    }
}
