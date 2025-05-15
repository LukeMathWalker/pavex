mod cloning_strategy;
pub mod validation;

pub use cloning_strategy::*;

use quote::ToTokens;
use quote::quote;

/// Add an attribute to catch public items that can't be imported
/// from outside the crate they were defined into.
///
/// This restriction can be disable via the `allow_unreachable_pub`
/// feature of this crate.
pub fn deny_unreachable_pub_attr() -> impl ToTokens {
    if cfg!(feature = "allow_unreachable_pub") {
        quote! {
            #[warn(unreachable_pub)]
        }
    } else {
        quote! {
            #[deny(unreachable_pub)]
        }
    }
}
