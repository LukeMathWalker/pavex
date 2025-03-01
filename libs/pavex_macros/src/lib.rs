use proc_macro::TokenStream;

mod config_profile;
mod path_params;

#[allow(non_snake_case)]
#[proc_macro_attribute]
pub fn PathParams(_metadata: TokenStream, input: TokenStream) -> TokenStream {
    path_params::path_params(input)
}

#[proc_macro_derive(ConfigProfile, attributes(pavex))]
pub fn derive_config_profile(input: TokenStream) -> TokenStream {
    config_profile::derive_config_profile(input)
}
