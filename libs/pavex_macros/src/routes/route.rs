use darling::util::Flag;
use pavexc_attr_parser::atoms::MethodArgument;
use quote::quote;

use crate::{
    fn_like::{Callable, CallableAnnotation, ImplContext},
    utils::AnnotationCodegen,
};

#[derive(darling::FromMeta, Debug, Clone)]
/// The available options for `#[pavex::route]`.
pub struct InputSchema {
    pub method: Option<MethodArgument>,
    pub path: String,
    pub allow: Option<RouteAllows>,
}

#[derive(darling::FromMeta, Debug, Clone)]
pub struct RouteAllows {
    non_standard_methods: Flag,
    any_method: Flag,
}

impl TryFrom<InputSchema> for Properties {
    type Error = darling::Error;

    fn try_from(input: InputSchema) -> Result<Self, Self::Error> {
        let InputSchema {
            path,
            method,
            allow,
        } = input;

        let allow_non_standard_methods = allow
            .as_ref()
            .map(|a| a.non_standard_methods.is_present())
            .unwrap_or(false);
        let allow_any_method = allow
            .as_ref()
            .map(|a| a.any_method.is_present())
            .unwrap_or(false);

        if let Some(method) = method.as_ref() {
            if allow_any_method {
                let msg = match method {
                    MethodArgument::Single(_) => {
                        "You can't use both `method` and `allow(any_method)` on the same route: \
                        either you accept a single method, or you accept them all.\n\
                        Remove one of the two arguments."
                    }
                    MethodArgument::Multiple(_) => {
                        "You can't use both `method` and `allow(any_method)` on the same route: \
                        either you accept a list of specific methods, or you accept them all.\n\
                        Remove one of the two arguments."
                    }
                };
                return Err(darling::Error::custom(msg));
            }

            let standard_methods = [
                "CONNECT", "GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS", "TRACE",
            ];
            if allow_non_standard_methods {
                match &method {
                    MethodArgument::Single(m) => {
                        if standard_methods.contains(&m.as_str()) {
                            let err = darling::Error::custom(format!(
                                "`{m}` is a standard HTTP method. Remove `allow(non_standard_methods)`."
                            ));
                            return Err(err);
                        }
                    }
                    MethodArgument::Multiple(vec) => {
                        if vec.iter().all(|m| standard_methods.contains(&m.as_str())) {
                            let err = darling::Error::custom("All the methods you specified are standard HTTP methods. Remove `allow(non_standard_methods)`.".to_string());
                            return Err(err);
                        }
                    }
                }
            } else {
                let error = |m: &str| {
                    darling::Error::custom(format!("`{m}` is not a standard HTTP method.",))
                };
                match &method {
                    MethodArgument::Single(m) => {
                        if !standard_methods.contains(&m.as_str()) {
                            return Err(error(m));
                        }
                    }
                    MethodArgument::Multiple(vec) => {
                        let mut errors = darling::Error::accumulator();
                        for m in vec {
                            if !standard_methods.contains(&m.as_str()) {
                                errors.push(error(m));
                            }
                        }
                        errors.finish()?;
                    }
                }
            }
        }

        Ok(Properties {
            path,
            method,
            allow_non_standard_methods,
            allow_any_method,
        })
    }
}

#[derive(darling::FromMeta, Debug, Clone, PartialEq, Eq)]
pub struct Properties {
    pub method: Option<MethodArgument>,
    pub path: String,
    pub allow_non_standard_methods: bool,
    pub allow_any_method: bool,
}

pub struct RouteAnnotation;

impl CallableAnnotation for RouteAnnotation {
    const PLURAL_COMPONENT_NAME: &str = "Request handlers";

    const ATTRIBUTE: &str = "#[pavex::route]";

    type InputSchema = InputSchema;

    fn codegen(
        _impl_: Option<ImplContext>,
        metadata: Self::InputSchema,
        _item: Callable,
    ) -> Result<AnnotationCodegen, proc_macro::TokenStream> {
        let properties = metadata
            .try_into()
            .map_err(|e: darling::Error| e.write_errors())?;
        Ok(emit(properties))
    }
}

/// Decorate the input with a `#[diagnostic::pavex::route]` attribute
/// that matches the provided properties.
fn emit(properties: Properties) -> AnnotationCodegen {
    let Properties {
        method,
        path,
        allow_non_standard_methods,
        allow_any_method,
    } = properties;

    let mut properties = quote! {
        path = #path,
    };

    if let Some(method) = method {
        properties.extend(quote! {
            method = #method,
        });
    }
    if allow_non_standard_methods {
        properties.extend(quote! {
            allow_non_standard_methods = true,
        });
    }
    if allow_any_method {
        properties.extend(quote! {
            allow_any_method = true,
        });
    }

    AnnotationCodegen {
        id_def: None,
        new_attributes: vec![syn::parse_quote! {
            #[diagnostic::pavex::route(#properties)]
        }],
    }
}
