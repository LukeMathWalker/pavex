use crate::utils::validation::must_be_public;
use crate::utils::{CloningStrategy, CloningStrategyFlags, deny_unreachable_pub_attr};
use darling::FromMeta;
use darling::util::Flag;
use proc_macro::TokenStream;
use quote::{ToTokens, quote};

#[derive(darling::FromMeta, Debug, Clone)]
/// The available options for `#[pavex::prebuilt]`.
pub struct InputSchema {
    pub clone_if_necessary: Flag,
    pub never_clone: Flag,
}

impl TryFrom<InputSchema> for Properties {
    type Error = darling::Error;

    fn try_from(input: InputSchema) -> Result<Self, Self::Error> {
        let InputSchema {
            clone_if_necessary,
            never_clone,
        } = input;
        let Ok(cloning_strategy) = CloningStrategyFlags {
            clone_if_necessary,
            never_clone,
        }
        .try_into() else {
            return Err(darling::Error::custom(
                "A prebuilt type can't have multiple cloning strategies. You can only specify *one* of `never_clone` and `clone_if_necessary`.",
            ));
        };

        Ok(Properties { cloning_strategy })
    }
}

#[derive(darling::FromMeta, Debug, Clone, PartialEq, Eq)]
pub struct Properties {
    pub cloning_strategy: Option<CloningStrategy>,
}

pub fn prebuilt(metadata: TokenStream, input: TokenStream) -> TokenStream {
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
    let Properties { cloning_strategy } = properties;
    let mut properties = quote! {};
    if let Some(cloning_strategy) = cloning_strategy {
        properties.extend(quote! {
            cloning_strategy = #cloning_strategy,
        });
    }

    let deny_unreachable_pub = deny_unreachable_pub_attr();

    let input: proc_macro2::TokenStream = input.into();
    quote! {
        #[diagnostic::pavex::prebuilt(#properties)]
        #deny_unreachable_pub
        #input
    }
    .into()
}

fn reject_invalid_input(input: TokenStream) -> Result<(), TokenStream> {
    let raw_item = match (
        syn::parse::<syn::ItemEnum>(input.clone()),
        syn::parse::<syn::ItemStruct>(input.clone()),
    ) {
        (Ok(item), _) => RawPrebuiltItem::Enum(item),
        (_, Ok(item)) => RawPrebuiltItem::Struct(item),
        _ => {
            return Err(syn::Error::new_spanned(
                proc_macro2::TokenStream::from(input),
                "#[pavex::prebuilt] can only be applied to enum and struct definitions.",
            )
            .to_compile_error()
            .into());
        }
    };
    must_be_public(
        "Prebuilt types",
        raw_item.visibility(),
        raw_item.ident(),
        &raw_item,
    )?;
    Ok(())
}

/// The raw item we parse prebuilt types from.
///
/// Its `ToTokens` representation can be used in error spans as the "default" option, unless
/// a more precise span is desired.
enum RawPrebuiltItem {
    Enum(syn::ItemEnum),
    Struct(syn::ItemStruct),
}

impl RawPrebuiltItem {
    fn ident(&self) -> &syn::Ident {
        match self {
            RawPrebuiltItem::Enum(item) => &item.ident,
            RawPrebuiltItem::Struct(item) => &item.ident,
        }
    }

    fn visibility(&self) -> &syn::Visibility {
        match self {
            RawPrebuiltItem::Enum(item) => &item.vis,
            RawPrebuiltItem::Struct(item) => &item.vis,
        }
    }
}

impl ToTokens for RawPrebuiltItem {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            RawPrebuiltItem::Enum(item) => {
                item.vis.to_tokens(tokens);
                item.enum_token.to_tokens(tokens);
                item.ident.to_tokens(tokens);
                item.generics.to_tokens(tokens);
            }
            RawPrebuiltItem::Struct(item) => {
                item.vis.to_tokens(tokens);
                item.struct_token.to_tokens(tokens);
                item.ident.to_tokens(tokens);
                item.generics.to_tokens(tokens);
            }
        }
    }
}
