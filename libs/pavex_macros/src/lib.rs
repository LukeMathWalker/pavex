use config::ConfigAnnotation;
use constructor::{RequestScopedAnnotation, SingletonAnnotation, TransientAnnotation};
use error_handler::ErrorHandlerAnnotation;
use error_observer::ErrorObserverAnnotation;
use fallback::FallbackAnnotation;
use middlewares::{PostProcessAnnotation, PreProcessAnnotation, WrapAnnotation};
use prebuilt::PrebuiltAnnotation;
use proc_macro::TokenStream;
use routes::{
    DeleteAnnotation, GetAnnotation, HeadAnnotation, OptionsAnnotation, PatchAnnotation,
    PostAnnotation, PutAnnotation, RouteAnnotation,
};
use utils::{fn_like::direct_entrypoint, type_like};

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

#[proc_macro_derive(ConfigProfile, attributes(pavex))]
pub fn derive_config_profile(input: TokenStream) -> TokenStream {
    config_profile::derive_config_profile(input)
}

#[proc_macro]
pub fn from(input: TokenStream) -> TokenStream {
    from::from_(input)
}

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
    type_like::entrypoint::<ConfigAnnotation>(metadata.into(), input.into())
}

#[proc_macro_attribute]
pub fn prebuilt(metadata: TokenStream, input: TokenStream) -> TokenStream {
    type_like::entrypoint::<PrebuiltAnnotation>(metadata.into(), input.into())
}

#[proc_macro_attribute]
pub fn error_observer(metadata: TokenStream, input: TokenStream) -> TokenStream {
    direct_entrypoint::<ErrorObserverAnnotation>(metadata.into(), input.into())
}

#[proc_macro_attribute]
pub fn error_handler(metadata: TokenStream, input: TokenStream) -> TokenStream {
    direct_entrypoint::<ErrorHandlerAnnotation>(metadata.into(), input.into())
}

#[proc_macro_attribute]
pub fn wrap(metadata: TokenStream, input: TokenStream) -> TokenStream {
    direct_entrypoint::<WrapAnnotation>(metadata.into(), input.into())
}

#[proc_macro_attribute]
pub fn pre_process(metadata: TokenStream, input: TokenStream) -> TokenStream {
    direct_entrypoint::<PreProcessAnnotation>(metadata.into(), input.into())
}

#[proc_macro_attribute]
pub fn post_process(metadata: TokenStream, input: TokenStream) -> TokenStream {
    direct_entrypoint::<PostProcessAnnotation>(metadata.into(), input.into())
}

#[proc_macro_attribute]
pub fn singleton(metadata: TokenStream, input: TokenStream) -> TokenStream {
    direct_entrypoint::<SingletonAnnotation>(metadata.into(), input.into())
}

#[proc_macro_attribute]
pub fn transient(metadata: TokenStream, input: TokenStream) -> TokenStream {
    direct_entrypoint::<TransientAnnotation>(metadata.into(), input.into())
}

#[proc_macro_attribute]
pub fn request_scoped(metadata: TokenStream, input: TokenStream) -> TokenStream {
    direct_entrypoint::<RequestScopedAnnotation>(metadata.into(), input.into())
}

#[proc_macro_attribute]
pub fn route(metadata: TokenStream, input: TokenStream) -> TokenStream {
    direct_entrypoint::<RouteAnnotation>(metadata.into(), input.into())
}

#[proc_macro_attribute]
pub fn get(metadata: TokenStream, input: TokenStream) -> TokenStream {
    direct_entrypoint::<GetAnnotation>(metadata.into(), input.into())
}

#[proc_macro_attribute]
pub fn post(metadata: TokenStream, input: TokenStream) -> TokenStream {
    direct_entrypoint::<PostAnnotation>(metadata.into(), input.into())
}

#[proc_macro_attribute]
pub fn put(metadata: TokenStream, input: TokenStream) -> TokenStream {
    direct_entrypoint::<PutAnnotation>(metadata.into(), input.into())
}

#[proc_macro_attribute]
pub fn delete(metadata: TokenStream, input: TokenStream) -> TokenStream {
    direct_entrypoint::<DeleteAnnotation>(metadata.into(), input.into())
}

#[proc_macro_attribute]
pub fn patch(metadata: TokenStream, input: TokenStream) -> TokenStream {
    direct_entrypoint::<PatchAnnotation>(metadata.into(), input.into())
}

#[proc_macro_attribute]
pub fn head(metadata: TokenStream, input: TokenStream) -> TokenStream {
    direct_entrypoint::<HeadAnnotation>(metadata.into(), input.into())
}

#[proc_macro_attribute]
pub fn options(metadata: TokenStream, input: TokenStream) -> TokenStream {
    direct_entrypoint::<OptionsAnnotation>(metadata.into(), input.into())
}

#[proc_macro_attribute]
pub fn fallback(metadata: TokenStream, input: TokenStream) -> TokenStream {
    direct_entrypoint::<FallbackAnnotation>(metadata.into(), input.into())
}
