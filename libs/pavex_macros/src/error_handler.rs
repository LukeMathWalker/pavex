use convert_case::{Case, Casing};
use darling::FromMeta as _;
use proc_macro::TokenStream;
use quote::{format_ident, quote, quote_spanned};
use syn::{Ident, ItemFn};

use crate::utils::{deny_unreachable_pub_attr, validation::must_be_public};

#[derive(darling::FromMeta, Debug, Clone)]
/// The available options for the error handler macro.
pub struct InputSchema {
    pub id: Option<syn::Ident>,
}

#[derive(darling::FromMeta, Debug, Clone, PartialEq, Eq)]
pub struct Properties {
    pub id: Option<syn::Ident>,
    pub error_ref_input_index: usize,
}

impl Properties {
    pub fn new(schema: InputSchema, error_ref_input_index: usize) -> Self {
        let InputSchema { id } = schema;
        Self {
            id,
            error_ref_input_index,
        }
    }
}

pub fn error_handler(
    self_ty: Option<&syn::Type>,
    metadata: proc_macro2::TokenStream,
    input: proc_macro2::TokenStream,
) -> Result<ErrorHandlerOutput, TokenStream> {
    let attr_name = "#[pavex::error_handler]";
    let func = reject_invalid_input(input.clone(), attr_name)?;
    let attrs = darling::ast::NestedMeta::parse_meta_list(metadata.into())
        .map_err(|e| e.to_compile_error())?;
    let schema = InputSchema::from_list(&attrs).map_err(|e| e.write_errors())?;
    let error_ref_index = find_error_ref_index(&func).map_err(|e| e.write_errors())?;
    let properties = Properties::new(schema, error_ref_index);
    Ok(emit(self_ty, func.sig.ident, properties))
}

fn reject_invalid_input(
    input: proc_macro2::TokenStream,
    macro_attr: &'static str,
) -> Result<ItemFn, TokenStream> {
    // Check if the input is a function
    let Ok(i) = syn::parse2::<syn::ItemFn>(input.clone()) else {
        // Neither ItemFn nor ImplItemFn - return an error
        let msg = format!("{macro_attr} can only be applied to free functions.");
        return Err(
            syn::Error::new_spanned(proc_macro2::TokenStream::from(input), msg)
                .to_compile_error()
                .into(),
        );
    };
    must_be_public("Error handlers", &i.vis, &i.sig.ident, &i.sig)?;
    Ok(i)
}

/// Returns the index of the input parameter annotated `#[px(error_ref)]`.
/// The annotation can be omitted if there is only one input parameter.
fn find_error_ref_index(func: &ItemFn) -> Result<usize, darling::Error> {
    use darling::FromAttributes;

    #[derive(FromAttributes, Debug, Clone)]
    #[darling(attributes(px))]
    struct InputAnnotation {
        error_ref: darling::util::Flag,
    }

    let inputs = &func.sig.inputs;
    let mut found = Vec::new();

    for (i, arg) in inputs.iter().enumerate() {
        let attrs = match arg {
            syn::FnArg::Receiver(receiver) => &receiver.attrs,
            syn::FnArg::Typed(pat_type) => &pat_type.attrs,
        };

        let annotation = InputAnnotation::from_attributes(attrs)?;
        if annotation.error_ref.is_present() {
            found.push(i);
        }
    }

    match (inputs.len(), found.len()) {
        (0, _) => Err(syn::Error::new(
            func.sig.paren_token.span.join(),
            "Error handlers must have at least one input parameter, a reference to the error type.",
        ).into()),
        (1, _) => Ok(0),        // single‐arg defaults to 0
        (_, 1) => Ok(found[0]), // exactly one annotation
        (_, 0) => Err(syn::Error::new(
            func.sig.paren_token.span.join(),
            "Mark the error reference input with `#[px(error_ref)]`.\n\
            Pavex can't automatically identify it if your error handler has two or more input parameters.",
        ).into()),
        (_, _) => Err(syn::Error::new(
            func.sig.paren_token.span.join(),
            "Only one input parameter may be annotated with #[px(error_ref)].",
        ).into())
    }
}

pub struct ErrorHandlerOutput {
    pub id_def: proc_macro2::TokenStream,
    pub new_attributes: Vec<syn::Attribute>,
}

/// Decorate the input with a `#[diagnostic::pavex::wrap]` attribute
/// that matches the provided properties.
fn emit(self_ty: Option<&syn::Type>, name: Ident, properties: Properties) -> ErrorHandlerOutput {
    let Properties {
        id,
        error_ref_input_index,
    } = properties;
    // Use the span of the function name if no identifier is provided.
    let id_span = id.as_ref().map(|id| id.span()).unwrap_or(name.span());

    let name = name.to_string();
    let handler_path = if let Some(syn::Type::Path(self_ty)) = self_ty {
        let ty_name = &self_ty
            .path
            .segments
            .last()
            .expect("The type path must contains at least one segment, the type name")
            .ident;
        format!("{}::{}", ty_name, name)
    } else {
        name
    };

    // If the user didn't specify an identifier, generate one based on the function name.
    let id = id.unwrap_or_else(|| {
        format_ident!(
            "{}",
            handler_path.replace("::", "_").to_case(Case::Constant)
        )
    });
    let properties = quote! {
        id = #id,
        error_ref_input_index = #error_ref_input_index
    };

    let id_docs = format!(
        r#"A strongly-typed id to add [`{handler_path}`] as an error handler to your Pavex application.

# Example

```rust,ignore
use pavex::blueprint::Blueprint;
// [...]
// ^ Import `{id}` here

let mut bp = Blueprint::new();
// Add `{handler_path}` as an error handler to your application.
bp.error_handler({id});
```"#
    );
    let id_def = quote_spanned! { id_span =>
        #[doc = #id_docs]
        pub const #id: ::pavex::blueprint::reflection::WithLocation<::pavex::blueprint::reflection::RawIdentifiers> =
            ::pavex::with_location!(::pavex::blueprint::reflection::RawIdentifiers {
                import_path: concat!(module_path!(), "::", #handler_path),
                macro_name: "error_handler",
            });
    };

    ErrorHandlerOutput {
        id_def,
        new_attributes: vec![
            syn::parse_quote! { #[diagnostic::pavex::error_handler(#properties)] },
            deny_unreachable_pub_attr(),
        ],
    }
}
