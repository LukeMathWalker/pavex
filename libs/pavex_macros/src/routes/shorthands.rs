use crate::utils::AnnotationCodegen;
use crate::utils::fn_like::{Callable, CallableAnnotation, ImplContext};
use crate::utils::id::{callable_id_def, default_id};
use quote::quote;
use syn::Ident;

#[derive(darling::FromMeta, Debug, Clone)]
/// The available options for route macros.
pub struct InputSchema {
    pub path: String,
    pub id: Option<Ident>,
    pub error_handler: Option<String>,
    pub allow: Option<ShorthandAllows>,
}

#[derive(darling::FromMeta, Debug, Clone)]
pub struct ShorthandAllows {
    error_fallback: darling::util::Flag,
}

impl TryFrom<InputSchema> for Properties {
    type Error = darling::Error;

    fn try_from(input: InputSchema) -> Result<Self, Self::Error> {
        let InputSchema {
            path,
            error_handler,
            id,
            allow,
        } = input;
        let allow_error_fallback = allow.as_ref().map(|a| a.error_fallback.is_present());
        Ok(Properties {
            path,
            id,
            error_handler,
            allow_error_fallback,
        })
    }
}

#[derive(darling::FromMeta, Debug, Clone, PartialEq, Eq)]
struct Properties {
    pub path: String,
    pub error_handler: Option<String>,
    pub id: Option<Ident>,
    pub allow_error_fallback: Option<bool>,
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
                    impl_: Option<ImplContext>,
                    metadata: Self::InputSchema,
                    item: Callable,
                ) -> Result<AnnotationCodegen, proc_macro::TokenStream> {
                    method_shorthand(impl_, item, Method::$method, metadata)
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
    impl_: Option<ImplContext>,
    item: Callable,
    method: Method,
    schema: InputSchema,
) -> Result<AnnotationCodegen, proc_macro::TokenStream> {
    let properties = schema
        .try_into()
        .map_err(|e: darling::Error| e.write_errors())?;
    Ok(emit(impl_, item, method, properties))
}

/// Decorate the input with a `#[diagnostic::pavex::route]` attribute
/// that matches the provided properties.
fn emit(
    impl_: Option<ImplContext>,
    item: Callable,
    method: Method,
    properties: Properties,
) -> AnnotationCodegen {
    let Properties {
        path,
        error_handler,
        id,
        allow_error_fallback,
    } = properties;

    let method_name = method.name();
    let id = id.unwrap_or_else(|| default_id(impl_.as_ref(), &item));
    let id_str = id.to_string();

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
    if let Some(allow_error_fallback) = allow_error_fallback {
        properties.extend(quote! {
            allow_error_fallback = #allow_error_fallback,
        });
    }
    AnnotationCodegen {
        id_def: Some(callable_id_def(
            &id,
            None,
            "route",
            "Route",
            "a route",
            "route",
            true,
            impl_.as_ref(),
            &item,
        )),
        new_attributes: vec![syn::parse_quote! {
            #[diagnostic::pavex::route(#properties)]
        }],
    }
}
