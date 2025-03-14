use darling::FromMeta;
use proc_macro::TokenStream;
use quote::{ToTokens, quote};

#[derive(darling::FromMeta, Debug, Clone, PartialEq, Eq)]
pub struct Properties {
    pub lifecycle: Lifecycle,
    pub cloning_strategy: Option<CloningStrategy>,
    pub error_handler: Option<String>,
}

#[derive(darling::FromMeta, Debug, Clone, PartialEq, Eq)]
#[darling(rename_all = "snake_case")]
pub enum Lifecycle {
    Singleton,
    RequestScoped,
    Transient,
}

impl ToTokens for Lifecycle {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            Lifecycle::Singleton => tokens.extend(quote! { singleton }),
            Lifecycle::RequestScoped => tokens.extend(quote! { request_scoped }),
            Lifecycle::Transient => tokens.extend(quote! { transient }),
        }
    }
}

#[derive(darling::FromMeta, Debug, Clone, PartialEq, Eq)]
#[darling(rename_all = "snake_case")]
pub enum CloningStrategy {
    CloneIfNecessary,
    NeverClone,
}

impl ToTokens for CloningStrategy {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            CloningStrategy::CloneIfNecessary => tokens.extend(quote! { clone_if_necessary }),
            CloningStrategy::NeverClone => tokens.extend(quote! { never_clone }),
        }
    }
}

pub fn constructor(metadata: TokenStream, input: TokenStream) -> TokenStream {
    let attrs = match darling::ast::NestedMeta::parse_meta_list(metadata.into()) {
        Ok(attrs) => attrs,
        Err(err) => return err.to_compile_error().into(),
    };
    let Properties {
        lifecycle,
        cloning_strategy,
        error_handler,
    } = match Properties::from_list(&attrs) {
        Ok(parsed) => parsed,
        Err(err) => return err.write_errors().into(),
    };
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
