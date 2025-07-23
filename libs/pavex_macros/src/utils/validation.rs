use proc_macro::TokenStream;
use quote::{ToTokens, quote};

/// Reject items (functions and methods) that are not public.
///
/// If the visibility is inherited (i.e. no visibility is specified),
/// we use `fallback_tokens` as the span shown in the error message.
pub fn must_be_public<T: ToTokens>(
    kind: &str,
    visibility: &syn::Visibility,
    ident_after_visibility: &syn::Ident,
    fallback_tokens: T,
) -> Result<(), TokenStream> {
    if matches!(visibility, syn::Visibility::Public(_)) {
        return Ok(());
    }

    let mut msg = format!(
        "{kind} must be public.\nMark `{ident_after_visibility}` as `pub`,"
    );
    let suffix = " and make sure it can be imported from outside your crate.";
    // If the visibility is inherited, there is no token we can "highlight".
    // We use the signature in that case to improve the quality of the error message.
    let e = if matches!(visibility, syn::Visibility::Inherited) {
        msg.push_str(suffix);
        syn::Error::new_spanned(fallback_tokens, msg)
    } else {
        msg.push_str(&format!(" instead of `{}`,", quote! { #visibility }));
        msg.push_str(suffix);
        syn::Error::new_spanned(visibility, msg)
    };
    Err(e.to_compile_error().into())
}
