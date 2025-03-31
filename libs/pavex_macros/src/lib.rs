use proc_macro::TokenStream;

mod config_profile;
mod constructor;
mod from;
mod path_params;

#[allow(non_snake_case)]
#[proc_macro_attribute]
pub fn PathParams(_metadata: TokenStream, input: TokenStream) -> TokenStream {
    path_params::path_params(input)
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

#[proc_macro_derive(ConfigProfile, attributes(pavex))]
pub fn derive_config_profile(input: TokenStream) -> TokenStream {
    config_profile::derive_config_profile(input)
}

#[proc_macro]
pub fn from(input: TokenStream) -> TokenStream {
    from::from_(input)
}
