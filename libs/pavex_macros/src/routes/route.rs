use darling::{FromMeta as _, util::Flag};
use pavexc_attr_parser::atoms::MethodArgument;
use proc_macro::TokenStream;
use quote::quote;
use syn::Ident;

use crate::utils::{deny_unreachable_pub_attr, validation::must_be_public};

#[derive(darling::FromMeta, Debug, Clone)]
/// The available options for `#[pavex::route]`.
pub struct InputSchema {
    pub method: Option<MethodArgument>,
    pub path: String,
    pub error_handler: Option<String>,
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
            error_handler,
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
            error_handler,
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
    pub error_handler: Option<String>,
    pub allow_non_standard_methods: bool,
    pub allow_any_method: bool,
}

pub fn route(metadata: TokenStream, input: TokenStream) -> TokenStream {
    let _name = match reject_invalid_input(input.clone(), "#[pavex::route]") {
        Ok(name) => name,
        Err(err) => return err,
    };
    let attrs = match darling::ast::NestedMeta::parse_meta_list(metadata.into()) {
        Ok(attrs) => attrs,
        Err(err) => return err.to_compile_error().into(),
    };
    let schema = match InputSchema::from_list(&attrs) {
        Ok(parsed) => parsed,
        Err(err) => return err.write_errors().into(),
    };
    let properties = match schema.try_into() {
        Ok(properties) => properties,
        Err(err) => {
            let err: darling::Error = err;
            return err.write_errors().into();
        }
    };
    emit(properties, input)
}

fn reject_invalid_input(
    input: TokenStream,
    macro_attr: &'static str,
) -> Result<Ident, TokenStream> {
    // Check if the input is a function
    let Ok(i) = syn::parse::<syn::ItemFn>(input.clone()) else {
        // Neither ItemFn nor ImplItemFn - return an error
        let msg = format!("{macro_attr} can only be applied to free functions.");
        return Err(
            syn::Error::new_spanned(proc_macro2::TokenStream::from(input), msg)
                .to_compile_error()
                .into(),
        );
    };
    must_be_public("Routes", &i.vis, &i.sig.ident, &i.sig)?;
    Ok(i.sig.ident)
}

/// Decorate the input with a `#[diagnostic::pavex::route]` attribute
/// that matches the provided properties.
fn emit(properties: Properties, input: TokenStream) -> TokenStream {
    let Properties {
        method,
        path,
        error_handler,
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
    if let Some(error_handler) = error_handler {
        properties.extend(quote! {
            error_handler = #error_handler,
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

    let deny_unreachable_pub = deny_unreachable_pub_attr();
    let input: proc_macro2::TokenStream = input.into();
    quote! {
        #[diagnostic::pavex::route(#properties)]
        #deny_unreachable_pub
        #input
    }
    .into()
}
