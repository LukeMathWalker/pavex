use cloning_strategy::CloningStrategy;
use darling::FromMeta;
use lifecycle::Lifecycle;
use proc_macro::TokenStream;
use quote::quote;

mod cloning_strategy;
mod lifecycle;

#[derive(darling::FromMeta, Debug, Clone, PartialEq, Eq)]
pub struct Properties {
    pub lifecycle: Lifecycle,
    pub cloning_strategy: Option<CloningStrategy>,
    pub error_handler: Option<String>,
}

#[derive(darling::FromMeta, Debug, Clone, PartialEq, Eq)]
/// The available options for `#[pavex::request_scoped]`, `#[pavex::transient]`
/// and `#[pavex::singleton]`.
/// Everything in [`Properties`], minus `lifecycle`.
pub struct ShorthandProperties {
    pub cloning_strategy: Option<CloningStrategy>,
    pub error_handler: Option<String>,
}

pub fn constructor(metadata: TokenStream, input: TokenStream) -> TokenStream {
    let attrs = match darling::ast::NestedMeta::parse_meta_list(metadata.into()) {
        Ok(attrs) => attrs,
        Err(err) => return err.to_compile_error().into(),
    };
    let properties = match Properties::from_list(&attrs) {
        Ok(parsed) => parsed,
        Err(err) => return err.write_errors().into(),
    };
    emit(properties, input)
}

/// An annotation for a request-scoped constructor.
pub fn request_scoped(metadata: TokenStream, input: TokenStream) -> TokenStream {
    shorthand(metadata, input, Lifecycle::RequestScoped)
}

/// An annotation for a transient constructor.
pub fn transient(metadata: TokenStream, input: TokenStream) -> TokenStream {
    shorthand(metadata, input, Lifecycle::Transient)
}

/// An annotation for a singleton constructor.
pub fn singleton(metadata: TokenStream, input: TokenStream) -> TokenStream {
    shorthand(metadata, input, Lifecycle::Singleton)
}

fn shorthand(metadata: TokenStream, input: TokenStream, lifecycle: Lifecycle) -> TokenStream {
    let attrs = match darling::ast::NestedMeta::parse_meta_list(metadata.into()) {
        Ok(attrs) => attrs,
        Err(err) => return err.to_compile_error().into(),
    };
    let ShorthandProperties {
        cloning_strategy,
        error_handler,
    } = match ShorthandProperties::from_list(&attrs) {
        Ok(parsed) => parsed,
        Err(err) => return err.write_errors().into(),
    };
    let properties = Properties {
        lifecycle,
        cloning_strategy,
        error_handler,
    };
    emit(properties, input)
}

/// Decorate the input with a `#[diagnostic::pavex::constructor]` attribute
/// that matches the provided properties.
fn emit(properties: Properties, input: TokenStream) -> TokenStream {
    let Properties {
        lifecycle,
        cloning_strategy,
        error_handler,
    } = properties;
    let mut properties = quote! {
        lifecycle = #lifecycle,
    };
    if let Some(cloning_strategy) = cloning_strategy {
        properties.extend(quote! {
            cloning_strategy = #cloning_strategy,
        });
    }
    if let Some(error_handler) = error_handler {
        properties.extend(quote! {
            error_handler = #error_handler,
        });
    }

    let input: proc_macro2::TokenStream = input.into();
    quote! {
        #[diagnostic::pavex::constructor(#properties)]
        #input
    }
    .into()
}
