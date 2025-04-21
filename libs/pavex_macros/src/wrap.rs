use darling::FromMeta as _;
use proc_macro::TokenStream;
use quote::{quote, quote_spanned};
use syn::Ident;

#[derive(darling::FromMeta, Debug, Clone)]
/// The available options for `#[pavex::wrap]`.
pub struct InputSchema {
    pub id: syn::Ident,
}

impl TryFrom<InputSchema> for Properties {
    type Error = darling::Error;

    fn try_from(input: InputSchema) -> Result<Self, Self::Error> {
        let InputSchema { id } = input;
        Ok(Properties { id })
    }
}

#[derive(darling::FromMeta, Debug, Clone, PartialEq, Eq)]
pub struct Properties {
    pub id: syn::Ident,
}

pub fn wrap(metadata: TokenStream, input: TokenStream) -> TokenStream {
    let name = match reject_invalid_input(input.clone(), "#[pavex::wrap]") {
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
    emit(name, properties, input)
}

fn reject_invalid_input(
    input: TokenStream,
    macro_attr: &'static str,
) -> Result<Ident, TokenStream> {
    // Check if the input is a function
    match syn::parse::<syn::ItemFn>(input.clone()) {
        Ok(i) => Ok(i.sig.ident),
        Err(_) => {
            // Neither ItemFn nor ImplItemFn - return an error
            let msg = format!("{macro_attr} can only be applied to free functions.");
            Err(
                syn::Error::new_spanned(proc_macro2::TokenStream::from(input), msg)
                    .to_compile_error()
                    .into(),
            )
        }
    }
}

/// Decorate the input with a `#[diagnostic::pavex::wrap]` attribute
/// that matches the provided properties.
fn emit(name: Ident, properties: Properties, input: TokenStream) -> TokenStream {
    let Properties { id } = properties;
    let properties = quote! {
        id = #id,
    };

    let id_span = id.span();
    let name = name.to_string();
    let id_def = quote_spanned! { id_span =>
        pub const #id: ::pavex::blueprint::reflection::WithLocation<::pavex::blueprint::reflection::RawIdentifiers> =
            ::pavex::with_location!(::pavex::blueprint::reflection::RawIdentifiers {
                import_path: #name,
                macro_name: "wrap",
            });
    };

    let input: proc_macro2::TokenStream = input.into();
    quote! {
        #id_def

        #[diagnostic::pavex::wrap(#properties)]
        #input
    }
    .into()
}
