use convert_case::{Case, Casing};
use quote::{format_ident, quote, quote_spanned};
use syn::Ident;

use crate::{
    fn_like::{Callable, CallableAnnotation, ImplContext},
    utils::AnnotationCodegen,
};

#[derive(darling::FromMeta, Debug, Clone)]
/// The available options for the error handler macro.
pub struct InputSchema {
    pub id: Option<syn::Ident>,
    /// Whether the error handler should be use as the default
    /// whenever an error of the matching type is emitted.
    ///
    /// If omitted, default to true.
    pub default: Option<bool>,
    pub pavex: Option<syn::Ident>,
}

#[derive(darling::FromMeta, Debug, Clone, PartialEq, Eq)]
pub struct Properties {
    pub id: Option<syn::Ident>,
    pub error_ref_input_index: usize,
    pub default: Option<bool>,
    pub pavex: Option<syn::Ident>,
}

impl Properties {
    pub fn new(schema: InputSchema, error_ref_input_index: usize) -> Self {
        let InputSchema { id, pavex, default } = schema;
        Self {
            id,
            default,
            error_ref_input_index,
            pavex,
        }
    }
}

pub struct ErrorHandlerAnnotation;

impl CallableAnnotation for ErrorHandlerAnnotation {
    const PLURAL_COMPONENT_NAME: &str = "Error handlers";

    const ATTRIBUTE: &str = "#[pavex::error_handler]";

    type InputSchema = InputSchema;

    fn codegen(
        impl_: Option<ImplContext>,
        metadata: Self::InputSchema,
        item: Callable,
    ) -> Result<AnnotationCodegen, proc_macro::TokenStream> {
        let error_ref_index = find_error_ref_index(&item).map_err(|e| e.write_errors())?;
        let properties = Properties::new(metadata, error_ref_index);
        Ok(emit(impl_.map(|i| i.self_ty), item.sig.ident, properties))
    }
}

/// Returns the index of the input parameter annotated `#[px(error_ref)]`.
/// The annotation can be omitted if there is only one input parameter.
fn find_error_ref_index(func: &Callable) -> Result<usize, darling::Error> {
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

/// Decorate the input with a `#[diagnostic::pavex::wrap]` attribute
/// that matches the provided properties.
fn emit(self_ty: Option<&syn::Type>, name: Ident, properties: Properties) -> AnnotationCodegen {
    let Properties {
        id,
        error_ref_input_index,
        pavex,
        default,
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
    let id_str = id.to_string();
    let mut properties = quote! {
        id = #id_str,
        error_ref_input_index = #error_ref_input_index,
    };
    if let Some(default) = default {
        properties.extend(quote! { default = #default, });
    }

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
    let pavex = match pavex {
        Some(c) => quote! { #c },
        None => quote! { ::pavex },
    };
    let id_def = quote_spanned! { id_span =>
        #[doc = #id_docs]
        pub const #id: #pavex::blueprint::raw::RawErrorHandler = #pavex::blueprint::raw::RawErrorHandler {
            coordinates: #pavex::blueprint::reflection::AnnotationCoordinates {
                id: #id_str,
                created_at: #pavex::created_at!(),
                macro_name: "error_handler",
            }
        };
    };

    AnnotationCodegen {
        id_def: Some(id_def),
        new_attributes: vec![
            syn::parse_quote! { #[diagnostic::pavex::error_handler(#properties)] },
        ],
    }
}
