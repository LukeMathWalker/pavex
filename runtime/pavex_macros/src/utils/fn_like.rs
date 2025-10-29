//! Machinery to abstract away the parsing and validation logic that's shared by all
//! Pavex components that accept function-like inputs.
use proc_macro2::TokenStream;
use quote::quote;
use syn::{ImplItemFn, visit_mut::VisitMut};

use crate::utils::{AnnotationCodegen, deny_unreachable_pub_attr, validation::must_be_public};

#[derive(Clone, Copy)]
/// Information about the `impl` block that this method belongs to.
pub struct ImplContext<'a> {
    /// The `Self` type.
    pub self_ty: &'a syn::Type,
    /// `true` if it's a trait implementation.
    pub is_trait_impl: bool,
}

/// Either a free function or a method.
pub struct Callable {
    pub vis: syn::Visibility,
    pub sig: syn::Signature,
}

impl Callable {
    /// Parse the macro input as either a function or a method.
    pub fn parse<M: CallableAnnotation>(
        input: TokenStream,
        impl_: Option<ImplContext>,
    ) -> Result<Self, proc_macro::TokenStream> {
        let (vis, sig) = if let Ok(item_fn) = syn::parse2::<syn::ItemFn>(input.clone()) {
            (item_fn.vis, item_fn.sig)
        } else if let Ok(impl_fn) = syn::parse2::<syn::ImplItemFn>(input.clone()) {
            (impl_fn.vis, impl_fn.sig)
        } else {
            let msg = format!(
                "{} can only be applied to functions and methods.",
                M::ATTRIBUTE
            );
            return Err(syn::Error::new_spanned(input, msg)
                .to_compile_error()
                .into());
        };

        let self_ = Self { vis, sig };

        let check_visibility = if let Some(impl_) = impl_ {
            !impl_.is_trait_impl
        } else {
            true
        };
        if check_visibility {
            must_be_public(
                M::PLURAL_COMPONENT_NAME,
                &self_.vis,
                &self_.sig.ident,
                &self_.sig,
            )?;
        }

        Ok(self_)
    }
}

/// A Pavex annotation that can only be applied to either function or methods.
pub trait CallableAnnotation {
    /// The word(s) to refer to multiple components of this kind.
    /// It must always be capitalized.
    ///
    /// E.g. "Constructors".
    const PLURAL_COMPONENT_NAME: &str;
    /// The attribute, with its fully qualified path.
    ///
    /// E.g. `#[pavex::constructor]`.
    const ATTRIBUTE: &str;
    /// The type encoding the expected structure of the macro arguments, flags and options.
    type InputSchema: darling::FromMeta;

    fn codegen(
        impl_: Option<ImplContext>,
        metadata: Self::InputSchema,
        item: Callable,
    ) -> Result<AnnotationCodegen, proc_macro::TokenStream>;
}

/// The entrypoint used when the input is expected to be free-standing—i.e. not a method
/// within an `impl` block.
pub fn direct_entrypoint<M: CallableAnnotation>(
    metadata: TokenStream,
    input: TokenStream,
) -> proc_macro::TokenStream {
    pub fn _inner<M: CallableAnnotation>(
        metadata: TokenStream,
        input: TokenStream,
    ) -> Result<TokenStream, proc_macro::TokenStream> {
        let callable = Callable::parse::<M>(input.clone(), None)?;
        let mut output = M::codegen(None, parse_metadata(metadata)?, callable)?;
        // We are always working with free functions here, so safe to push this attr.
        output.new_attributes.push(deny_unreachable_pub_attr());
        Ok(output.emit(input))
    }

    match _inner::<M>(metadata, input.clone()) {
        Ok(t) => t.into(),
        Err(error_tokens) => {
            let error_tokens = proc_macro2::TokenStream::from(error_tokens);
            reemit_with_input(error_tokens, input)
        }
    }
}

/// The entrypoint used when the input is expected to be a method
/// within a larger `impl` block.
pub fn method_entrypoint<M: CallableAnnotation>(
    impl_: ImplContext,
    metadata: TokenStream,
    input: ImplItemFn,
) -> Result<AnnotationCodegen, proc_macro::TokenStream> {
    let callable = Callable {
        vis: input.vis,
        sig: input.sig,
    };
    M::codegen(Some(impl_), parse_metadata(metadata)?, callable).map(|mut output| {
        if !impl_.is_trait_impl {
            output.new_attributes.push(deny_unreachable_pub_attr());
        }
        output
    })
}

fn parse_metadata<T: darling::FromMeta>(metadata: TokenStream) -> Result<T, TokenStream> {
    let attrs =
        darling::ast::NestedMeta::parse_meta_list(metadata).map_err(|e| e.to_compile_error())?;
    T::from_list(&attrs).map_err(|e| e.write_errors())
}

fn reemit_with_input(
    error_tokens: proc_macro2::TokenStream,
    input: proc_macro2::TokenStream,
) -> proc_macro::TokenStream {
    if let Ok(mut item) = syn::parse2::<syn::Item>(input.clone()) {
        crate::utils::PxStripper.visit_item_mut(&mut item);
        return quote! { #error_tokens #item }.into();
    }
    if let Ok(mut method) = syn::parse2::<syn::ImplItemFn>(input.clone()) {
        crate::utils::PxStripper.visit_impl_item_fn_mut(&mut method);
        return quote! { #error_tokens #method }.into();
    }
    if let Ok(mut trait_fn) = syn::parse2::<syn::TraitItemFn>(input.clone()) {
        crate::utils::PxStripper.visit_trait_item_fn_mut(&mut trait_fn);
        return quote! { #error_tokens #trait_fn }.into();
    }
    if let Ok(mut foreign_fn) = syn::parse2::<syn::ForeignItemFn>(input.clone()) {
        crate::utils::PxStripper.visit_foreign_item_fn_mut(&mut foreign_fn);
        return quote! { #error_tokens #foreign_fn }.into();
    }
    quote! { #error_tokens #input }.into()
}
