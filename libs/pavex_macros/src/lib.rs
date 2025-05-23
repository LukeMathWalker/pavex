use error_handler::ErrorHandlerOutput;
use proc_macro::TokenStream;
use quote::quote;
use routes::Method;
use syn::visit_mut::VisitMut;
use utils::PxStripper;

mod config;
mod config_profile;
mod constructor;
mod error_handler;
mod error_observer;
mod fallback;
mod from;
mod methods;
mod middlewares;
mod path_params;
mod prebuilt;
mod routes;
pub(crate) mod utils;

#[allow(non_snake_case)]
#[proc_macro_attribute]
pub fn PathParams(_metadata: TokenStream, input: TokenStream) -> TokenStream {
    path_params::path_params(input)
}

#[proc_macro_attribute]
pub fn methods(metadata: TokenStream, input: TokenStream) -> TokenStream {
    match methods::methods(metadata, input) {
        Ok(t) | Err(t) => t,
    }
}

#[proc_macro_attribute]
pub fn config(metadata: TokenStream, input: TokenStream) -> TokenStream {
    config::config(metadata, input)
}

#[proc_macro_attribute]
pub fn prebuilt(metadata: TokenStream, input: TokenStream) -> TokenStream {
    prebuilt::prebuilt(metadata, input)
}

#[proc_macro_attribute]
pub fn error_observer(metadata: TokenStream, input: TokenStream) -> TokenStream {
    error_observer::error_observer(metadata, input)
}

#[proc_macro_attribute]
pub fn error_handler(metadata: TokenStream, input: TokenStream) -> TokenStream {
    let input: proc_macro2::TokenStream = input.into();
    match error_handler::error_handler(None, metadata.into(), input.clone()) {
        Ok(ErrorHandlerOutput {
            id_def,
            new_attributes,
        }) => {
            let mut input: syn::Item = syn::parse2(input).expect("Input is not a valid syn::Item");
            PxStripper.visit_item_mut(&mut input);
            quote! {
                #id_def

                #(#new_attributes)*
                #input
            }
            .into()
        }
        Err(t) => t,
    }
}

#[proc_macro_attribute]
pub fn wrap(metadata: TokenStream, input: TokenStream) -> TokenStream {
    middlewares::wrap(metadata, input)
}

#[proc_macro_attribute]
pub fn pre_process(metadata: TokenStream, input: TokenStream) -> TokenStream {
    middlewares::pre_process(metadata, input)
}

#[proc_macro_attribute]
pub fn post_process(metadata: TokenStream, input: TokenStream) -> TokenStream {
    middlewares::post_process(metadata, input)
}

#[proc_macro_attribute]
pub fn constructor(metadata: TokenStream, input: TokenStream) -> TokenStream {
    constructor::constructor(metadata, input)
}

#[proc_macro_attribute]
pub fn singleton(metadata: TokenStream, input: TokenStream) -> TokenStream {
    constructor::singleton(metadata, input)
}

#[proc_macro_attribute]
pub fn transient(metadata: TokenStream, input: TokenStream) -> TokenStream {
    constructor::transient(metadata, input)
}

#[proc_macro_attribute]
pub fn request_scoped(metadata: TokenStream, input: TokenStream) -> TokenStream {
    constructor::request_scoped(metadata, input)
}

#[proc_macro_attribute]
pub fn route(metadata: TokenStream, input: TokenStream) -> TokenStream {
    routes::route(metadata, input)
}

#[proc_macro_attribute]
pub fn get(metadata: TokenStream, input: TokenStream) -> TokenStream {
    routes::method_shorthand(Method::Get, metadata, input)
}

#[proc_macro_attribute]
pub fn post(metadata: TokenStream, input: TokenStream) -> TokenStream {
    routes::method_shorthand(Method::Post, metadata, input)
}

#[proc_macro_attribute]
pub fn put(metadata: TokenStream, input: TokenStream) -> TokenStream {
    routes::method_shorthand(Method::Put, metadata, input)
}

#[proc_macro_attribute]
pub fn delete(metadata: TokenStream, input: TokenStream) -> TokenStream {
    routes::method_shorthand(Method::Delete, metadata, input)
}

#[proc_macro_attribute]
pub fn patch(metadata: TokenStream, input: TokenStream) -> TokenStream {
    routes::method_shorthand(Method::Patch, metadata, input)
}

#[proc_macro_attribute]
pub fn head(metadata: TokenStream, input: TokenStream) -> TokenStream {
    routes::method_shorthand(Method::Head, metadata, input)
}

#[proc_macro_attribute]
pub fn options(metadata: TokenStream, input: TokenStream) -> TokenStream {
    routes::method_shorthand(Method::Options, metadata, input)
}

#[proc_macro_derive(ConfigProfile, attributes(pavex))]
pub fn derive_config_profile(input: TokenStream) -> TokenStream {
    config_profile::derive_config_profile(input)
}

#[proc_macro]
pub fn from(input: TokenStream) -> TokenStream {
    from::from_(input)
}

#[proc_macro_attribute]
pub fn fallback(metadata: TokenStream, input: TokenStream) -> TokenStream {
    fallback::fallback(metadata, input)
}
