use proc_macro::TokenStream;
use quote::quote;

/// Reject items (functions and methods) that are not public.
pub fn must_be_public(
    kind: &str,
    vis: &syn::Visibility,
    sig: &syn::Signature,
) -> Result<(), TokenStream> {
    if matches!(vis, syn::Visibility::Public(_)) {
        return Ok(());
    }

    let mut msg = format!("{kind} must be public.\nMark `{}` as `pub`,", sig.ident);
    let suffix = " and make sure it can be imported from outside your crate.";
    // If the visibility is inherited, there is no token we can "highlight".
    // We use the signature in that case to improve the quality of the error message.
    let e = if matches!(vis, syn::Visibility::Inherited) {
        msg.push_str(suffix);
        syn::Error::new_spanned(sig, msg)
    } else {
        msg.push_str(&format!(" instead of `{}`,", quote! { #vis }));
        msg.push_str(suffix);
        syn::Error::new_spanned(vis, msg)
    };
    Err(e.to_compile_error().into())
}
