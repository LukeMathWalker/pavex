use proc_macro::TokenStream;

use quote::quote;
use syn::punctuated::Punctuated;
use syn::{parse_macro_input, Attribute, Data, DeriveInput, Error, GenericParam, Token};

#[allow(non_snake_case)]
#[proc_macro_attribute]
pub fn RouteParams(_metadata: TokenStream, input: TokenStream) -> TokenStream {
    let mut ast = parse_macro_input!(input as DeriveInput);
    if let Err(mut e) = reject_serde_attributes(&ast) {
        // We emit both the error AND the original struct.
        // This is useful to avoid spurious "the type doesn't exist anymore" additional errors
        e.extend(vec![TokenStream::from(quote! { #ast })]);
        return e;
    }
    ast.attrs
        .push(syn::parse_quote!(#[derive(serde::Serialize, serde::Deserialize)]));

    let generics_with_bounds = &ast.generics;
    let generics_without_bounds = &ast
        .generics
        .params
        .iter()
        .map(|p| match p {
            GenericParam::Lifetime(l) => {
                let l = &l.lifetime;
                quote! { #l }
            }
            GenericParam::Type(t) => {
                let i = &t.ident;
                quote! { #i }
            }
            GenericParam::Const(c) => {
                let i = &c.ident;
                quote! { #i }
            }
        })
        .collect::<Punctuated<_, Token![,]>>();
    let struct_name = &ast.ident;
    let expanded = quote! {
        #ast

        impl #generics_with_bounds pavex::serialization::StructuralDeserialize for #struct_name < #generics_without_bounds > {}
    };

    TokenStream::from(expanded)
}

fn reject_serde_attributes(ast: &DeriveInput) -> Result<(), TokenStream> {
    for attr in &ast.attrs {
        reject_serde_attribute(attr)?;
    }
    match &ast.data {
        Data::Struct(data) => {
            for field in &data.fields {
                for attr in &field.attrs {
                    reject_serde_attribute(attr)?;
                }
            }
        }
        Data::Enum(data) => {
            for variant in &data.variants {
                for attr in &variant.attrs {
                    reject_serde_attribute(attr)?;
                }
                for field in &variant.fields {
                    for attr in &field.attrs {
                        reject_serde_attribute(attr)?;
                    }
                }
            }
        }
        Data::Union(union) => {
            for field in &union.fields.named {
                for attr in &field.attrs {
                    reject_serde_attribute(attr)?;
                }
            }
        }
    }
    Ok(())
}

/// We don't want to allow `serde` attributes on the top-level struct or any of its fields,
/// because we rely on `serde`'s default behaviour to determine, at code-generartion time,
/// if the route params can be deserialized from the URL of the incoming request.
fn reject_serde_attribute(attr: &Attribute) -> Result<(), TokenStream> {
    let err_msg = "`RouteParams` does not support `serde` attributes on the top-level struct or any of its fields.\n\n\
      `RouteParams` takes care of deriving `serde::Serialize` and `serde::Deserialize` for your struct, using the default \
       configuration. This allow Pavex to determine, at code-generation time, if the route params can \
       be successfully extracted from the URL of incoming requests for the relevant routes (e.g. do you \
       have a named field that doesn't map to any of the registered route parameters?).\n\n\
       If the default `serde` configuration won't work for your case, you should not derive `RouteParams` and \
       opt instead for implementing `serde::Serialize` and `serde::Deserialize` directly for your struct (either \
       manually or using a derive with custom attributes).\nKeep in mind that by going down this route \
       you give up compile-time checking of the route parameters!";
    if let Some(ident) = attr.path().get_ident() {
        if ident == "serde" {
            return Err(Error::new_spanned(attr, err_msg)
                .into_compile_error()
                .into());
        }
    }
    Ok(())
}
