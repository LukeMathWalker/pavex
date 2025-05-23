use proc_macro::TokenStream;
use quote::{ToTokens, quote};
use syn::{Attribute, ImplItem, visit_mut::VisitMut};

use crate::{
    error_handler::{ErrorHandlerOutput, error_handler},
    utils::PxStripper,
};

pub fn methods(_metadata: TokenStream, input: TokenStream) -> Result<TokenStream, TokenStream> {
    let mut impl_: syn::ItemImpl = syn::parse(input).map_err(|e| e.into_compile_error())?;
    let mut new_items: Vec<proc_macro2::TokenStream> = Vec::new();

    for item in impl_.items.iter_mut() {
        let ImplItem::Fn(method) = item else {
            continue;
        };
        let Some(attr_index) = method
            .attrs
            .iter()
            .enumerate()
            .find_map(|(i, a)| is_error_handler_attr(a).then_some(i))
        else {
            continue;
        };
        let attr = method.attrs.remove(attr_index);

        let metadata = match attr.meta {
            // No arguments, just the path—e.g. `#[error_handler]`.
            syn::Meta::Path(_) => proc_macro2::TokenStream::new(),
            // Arguments within parenthesis, that we want to extract and forward.
            syn::Meta::List(meta_list) => meta_list.tokens.to_token_stream(),
            syn::Meta::NameValue(_) => unreachable!(),
        };

        let ErrorHandlerOutput {
            id_def,
            new_attributes,
        } = error_handler(Some(&impl_.self_ty), metadata, method.to_token_stream())?;

        new_items.push(id_def);
        method.attrs.extend(new_attributes);
    }

    PxStripper.visit_item_impl_mut(&mut impl_);

    Ok(quote! {
        #(#new_items)*

        #impl_
    }
    .into())
}

fn is_error_handler_attr(attr: &Attribute) -> bool {
    // Fast‐path for single-segment import form #[error_handler]
    if attr.path().is_ident("error_handler") {
        return true;
    }

    // Multi-segment form #[pavex::error_handler]
    let mut segments = attr.path().segments.iter();
    if let (Some(first), Some(second)) = (segments.next(), segments.next()) {
        if first.ident == "pavex"
               && second.ident == "error_handler"
               // and *no* further segments
               && segments.next().is_none()
        {
            return true;
        }
    }

    false
}
