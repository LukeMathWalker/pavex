mod cloning_strategy;
mod px_stripper;

pub mod fn_like;
pub mod type_like;
pub mod validation;
pub use cloning_strategy::*;
pub use px_stripper::PxStripper;

use proc_macro2::TokenStream;
use quote::quote;

/// The outcome of the code generation process for a Pavex component annotation.
pub struct AnnotationCodegen {
    /// The definition of a strongly-typed handle pointing at the annotated component.
    /// It can be `None` for component kinds that don't require an id—e.g. constructors.
    pub id_def: Option<proc_macro2::TokenStream>,
    /// Attributes to be applied to the annotated component.
    pub new_attributes: Vec<syn::Attribute>,
}

impl AnnotationCodegen {
    pub fn emit(&self, input: TokenStream) -> TokenStream {
        use syn::visit_mut::VisitMut as _;

        let Self {
            id_def,
            new_attributes,
        } = self;
        let mut input: syn::Item = syn::parse2(input).expect("Input is not a valid syn::Item");
        PxStripper.visit_item_mut(&mut input);
        quote! {
            #id_def

            #(#new_attributes)*
            #input
        }
    }
}

/// Add an attribute to catch public items that can't be imported
/// from outside the crate they were defined into.
///
/// This restriction can be disable via the `allow_unreachable_pub`
/// feature of this crate.
pub fn deny_unreachable_pub_attr() -> syn::Attribute {
    if cfg!(feature = "allow_unreachable_pub") {
        syn::parse_quote! {
            #[warn(unreachable_pub)]
        }
    } else {
        syn::parse_quote! {
            #[deny(unreachable_pub)]
        }
    }
}
