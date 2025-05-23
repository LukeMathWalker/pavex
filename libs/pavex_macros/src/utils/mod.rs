mod cloning_strategy;
mod px_stripper;
pub mod validation;

pub use cloning_strategy::*;
pub use px_stripper::PxStripper;

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
