use crate::utils::{CloningStrategy, CloningStrategyFlags};
use darling::FromMeta;
use darling::util::Flag;
use proc_macro::TokenStream;
use quote::quote;

#[derive(darling::FromMeta, Debug, Clone)]
/// The available options for `#[pavex::config]`.
pub struct InputSchema {
    pub key: String,
    pub clone_if_necessary: Flag,
    pub never_clone: Flag,
    pub default_if_missing: Flag,
    pub include_if_unused: Flag,
}

impl TryFrom<InputSchema> for Properties {
    type Error = darling::Error;

    fn try_from(input: InputSchema) -> Result<Self, Self::Error> {
        let InputSchema {
            key,
            clone_if_necessary,
            never_clone,
            default_if_missing,
            include_if_unused,
        } = input;
        let Ok(cloning_strategy) = CloningStrategyFlags {
            clone_if_necessary,
            never_clone,
        }
        .try_into() else {
            return Err(darling::Error::custom(
                "A configuration type can't have multiple cloning strategies. You can only specify *one* of `never_clone` and `clone_if_necessary`.",
            ));
        };

        Ok(Properties {
            key,
            cloning_strategy,
            default_if_missing: default_if_missing.is_present(),
            include_if_unused: include_if_unused.is_present(),
        })
    }
}

#[derive(darling::FromMeta, Debug, Clone, PartialEq, Eq)]
pub struct Properties {
    pub key: String,
    pub cloning_strategy: Option<CloningStrategy>,
    pub default_if_missing: bool,
    pub include_if_unused: bool,
}

pub fn config(metadata: TokenStream, input: TokenStream) -> TokenStream {
    if let Err(e) = reject_invalid_input(input.clone()) {
        return e;
    }
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

/// Decorate the input with a `#[diagnostic::pavex::config]` attribute
/// that matches the provided properties.
fn emit(properties: Properties, input: TokenStream) -> TokenStream {
    let Properties {
        key,
        cloning_strategy,
        default_if_missing,
        include_if_unused,
    } = properties;
    let mut properties = quote! {
        key = #key,
    };
    if let Some(cloning_strategy) = cloning_strategy {
        properties.extend(quote! {
            cloning_strategy = #cloning_strategy,
        });
    }
    if default_if_missing {
        properties.extend(quote! {
            default_if_missing = true,
        });
    }
    if include_if_unused {
        properties.extend(quote! {
            include_if_unused = true,
        });
    }

    let input: proc_macro2::TokenStream = input.into();
    quote! {
        #[diagnostic::pavex::config(#properties)]
        #[deny(unreachable_pub)]
        #input
    }
    .into()
}

fn reject_invalid_input(input: TokenStream) -> Result<(), TokenStream> {
    // Check if the input is an enum or a struct.
    if syn::parse::<syn::ItemEnum>(input.clone()).is_ok()
        || syn::parse::<syn::ItemStruct>(input.clone()).is_ok()
    {
        return Ok(());
    }

    // Neither—return an error.
    Err(syn::Error::new_spanned(
        proc_macro2::TokenStream::from(input),
        "#[pavex::config] can only be applied to enum and struct definitions.",
    )
    .to_compile_error()
    .into())
}
