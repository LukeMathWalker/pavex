use convert_case::{Case, Casing};
use darling::FromMeta as _;
use proc_macro::TokenStream;
use quote::{format_ident, quote, quote_spanned};
use syn::Ident;

use crate::utils::{deny_unreachable_pub_attr, validation::must_be_public};

#[derive(darling::FromMeta, Debug, Clone)]
/// The available options for the error observer macro.
pub struct InputSchema {
    pub id: Option<syn::Ident>,
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
    pub id: Option<syn::Ident>,
}

pub fn error_observer(metadata: TokenStream, input: TokenStream) -> TokenStream {
    let attr_name = "#[pavex::error_observer]";
    let name = match reject_invalid_input(input.clone(), attr_name) {
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
    let Properties { id } = properties;
    // Use the span of the function name if no identifier is provided.
    let id_span = id.as_ref().map(|id| id.span()).unwrap_or(name.span());

    let name = name.to_string();

    // If the user didn't specify an identifier, generate one based on the function name.
    let id = id.unwrap_or_else(|| format_ident!("{}", name.to_case(Case::Constant)));
    let properties = quote! {
        id = #id,
    };

    let id_docs = format!(
        r#"A strongly-typed id to add [`{name}`] as an error observer to your Pavex application.

# Example

```rust,ignore
use pavex::blueprint::Blueprint;
// [...]
// ^ Import `{id}` here

let mut bp = Blueprint::new();
// Add `{name}` as an error observer to your application.
bp.error_observer({id});
```"#
    );
    let id_def = quote_spanned! { id_span =>
        #[doc = #id_docs]
        pub const #id: ::pavex::blueprint::reflection::WithLocation<::pavex::blueprint::reflection::RawIdentifiers> =
            ::pavex::with_location!(::pavex::blueprint::reflection::RawIdentifiers {
                import_path: concat!(module_path!(), "::", #name),
                macro_name: "error_observer",
            });
    };

    let deny_unreachable_pub = deny_unreachable_pub_attr();

    let input: proc_macro2::TokenStream = input.into();
    quote! {
        #id_def

        #[diagnostic::pavex::error_observer(#properties)]
        #deny_unreachable_pub
        #input
    }
    .into()
}
