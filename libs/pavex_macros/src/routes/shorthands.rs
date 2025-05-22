use darling::FromMeta as _;
use proc_macro::TokenStream;
use quote::quote;
use syn::Ident;

use crate::utils::{deny_unreachable_pub_attr, validation::must_be_public};

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
pub struct Properties {
    pub path: String,
    pub error_handler: Option<String>,
}

#[derive(Clone, Copy)]
pub enum Method {
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

    pub fn attr(&self) -> &'static str {
        use Method::*;
        match self {
            Get => "#[pavex::get]",
            Delete => "#[pavex::delete]",
            Head => "#[pavex::head]",
            Options => "#[pavex::options]",
            Patch => "#[pavex::patch]",
            Post => "#[pavex::post]",
            Put => "#[pavex::put]",
        }
    }
}

pub fn method_shorthand(method: Method, metadata: TokenStream, input: TokenStream) -> TokenStream {
    let _name = match reject_invalid_input(input.clone(), method.attr()) {
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
    emit(method, properties, input)
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
fn emit(method: Method, properties: Properties, input: TokenStream) -> TokenStream {
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

    let deny_unreachable_pub = deny_unreachable_pub_attr();
    let input: proc_macro2::TokenStream = input.into();
    quote! {
        #[diagnostic::pavex::route(#properties)]
        #deny_unreachable_pub
        #input
    }
    .into()
}
