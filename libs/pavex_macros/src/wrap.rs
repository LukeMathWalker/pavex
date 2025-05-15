use convert_case::{Case, Casing};
use darling::FromMeta as _;
use proc_macro::TokenStream;
use quote::{format_ident, quote, quote_spanned};
use syn::Ident;

use crate::utils::{deny_unreachable_pub_attr, validation::must_be_public};

#[derive(darling::FromMeta, Debug, Clone)]
/// The available options for `#[pavex::wrap]`.
pub struct InputSchema {
    pub id: Option<syn::Ident>,
    pub error_handler: Option<String>,
}

impl TryFrom<InputSchema> for Properties {
    type Error = darling::Error;

    fn try_from(input: InputSchema) -> Result<Self, Self::Error> {
        let InputSchema { id, error_handler } = input;
        Ok(Properties { id, error_handler })
    }
}

#[derive(darling::FromMeta, Debug, Clone, PartialEq, Eq)]
pub struct Properties {
    pub id: Option<syn::Ident>,
    pub error_handler: Option<String>,
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
    let Ok(i) = syn::parse::<syn::ItemFn>(input.clone()) else {
        // Neither ItemFn nor ImplItemFn - return an error
        let msg = format!("{macro_attr} can only be applied to free functions.");
        return Err(
            syn::Error::new_spanned(proc_macro2::TokenStream::from(input), msg)
                .to_compile_error()
                .into(),
        );
    };
    must_be_public("Middlewares", &i.vis, &i.sig.ident, &i.sig)?;
    Ok(i.sig.ident)
}

/// Decorate the input with a `#[diagnostic::pavex::wrap]` attribute
/// that matches the provided properties.
fn emit(name: Ident, properties: Properties, input: TokenStream) -> TokenStream {
    let Properties { id, error_handler } = properties;
    // Use the span of the function name if no identifier is provided.
    let id_span = id.as_ref().map(|id| id.span()).unwrap_or(name.span());

    let name = name.to_string();

    // If the user didn't specify an identifier, generate one based on the function name.
    let id = id.unwrap_or_else(|| format_ident!("{}_ID", name.to_case(Case::Constant)));
    let mut properties = quote! {
        id = #id,
    };

    if let Some(error_handler) = error_handler {
        properties.extend(quote! {
            error_handler = #error_handler,
        });
    }

    let id_docs = format!(
        r#"A strongly-typed id to add [`{name}`] as a wrapping middleware to your Pavex application.

# Example

```rust,ignore
use pavex::blueprint::Blueprint;
// [...]
// ^ Import `{id}` here

let mut bp = Blueprint::new();
// Add `{name}` as a wrapping middleware to your application.
bp.wrap({id});
```"#
    );
    let id_def = quote_spanned! { id_span =>
        #[doc = #id_docs]
        pub const #id: ::pavex::blueprint::reflection::WithLocation<::pavex::blueprint::reflection::RawIdentifiers> =
            ::pavex::with_location!(::pavex::blueprint::reflection::RawIdentifiers {
                import_path: concat!(module_path!(), "::", #name),
                macro_name: "wrap",
            });
    };

    let deny_unreachable_pub = deny_unreachable_pub_attr();

    let input: proc_macro2::TokenStream = input.into();
    quote! {
        #id_def

        #[diagnostic::pavex::wrap(#properties)]
        #deny_unreachable_pub
        #input
    }
    .into()
}
