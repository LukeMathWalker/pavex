use proc_macro::TokenStream;
use quote::{ToTokens, quote};
use syn::{Error, Token, ext::IdentExt, parse_macro_input, punctuated::Punctuated};

pub fn from_(input: TokenStream) -> TokenStream {
    // Parse input as a comma-separated list of paths or the wildcard '*'
    let paths =
        parse_macro_input!(input with Punctuated::<ModulePath, Token![,]>::parse_terminated);

    if paths.is_empty() {
        return syn::Error::new_spanned(
            &paths,
            "You must specify at least one source when invoking `from!`",
        )
        .to_compile_error()
        .into();
    }

    // Check if the input contains a `*`
    let contains_wildcard = paths.iter().any(|p| matches!(p, ModulePath::Wildcard(_)));
    if contains_wildcard {
        return if paths.len() == 1 {
            // If only `*` was provided, return `Sources::All`
            quote! {
                ::pavex::with_location!(::pavex::blueprint::reflection::Sources::All)
            }
            .into()
        } else {
            syn::Error::new_spanned(
                &paths,
                "The wildcard source, `*`, can't be combined with other module paths.\n\
                `*` will automatically include all local modules and all direct dependencies of the current crate.",
            )
            .to_compile_error()
            .into()
        };
    };

    let mut sources = Vec::new();
    let mut error: Option<syn::Error> = None;
    for path in paths {
        match validate_path(&path) {
            Ok(source) => sources.push(source),
            Err(e) => {
                if let Some(old) = &mut error {
                    old.combine(e);
                } else {
                    error = Some(e);
                }
            }
        }
    }
    match error {
        Some(err) => err.to_compile_error().into(),
        None => quote! {
            ::pavex::with_location!(::pavex::blueprint::reflection::Sources::Some(vec![#(#sources.into()),*]))
        }
        .into(),
    }
}

/// A valid element for the sequence that `from!` expects as input.
enum ModulePath {
    /// A wildcard module path, e.g. `*`.
    Wildcard(Token![*]),
    /// A path to a module, e.g. `crate::foo::bar`.
    Path(Punctuated<syn::Ident, Token![::]>),
}

impl syn::parse::Parse for ModulePath {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(Token![*]) {
            input.parse().map(ModulePath::Wildcard)
        } else {
            Punctuated::<syn::Ident, Token![::]>::parse_separated_nonempty_with(
                input,
                // We use `parse_any` to allow keywords (e.g. `crate` or `super` or `self`)
                // which would otherwise be rejected.
                syn::Ident::parse_any,
            )
            .map(ModulePath::Path)
        }
    }
}

impl ToTokens for ModulePath {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            ModulePath::Wildcard(wildcard) => wildcard.to_tokens(tokens),
            ModulePath::Path(path) => path.to_tokens(tokens),
        }
    }
}

/// Validate a given path and convert it into `Source`
fn validate_path(path: &ModulePath) -> Result<String, syn::Error> {
    let ModulePath::Path(path) = path else {
        unreachable!()
    };
    if path.is_empty() {
        return Err(Error::new_spanned(path, "Empty paths are not allowed"));
    }
    Ok(path
        .iter()
        .map(|segment| segment.to_string())
        .collect::<Vec<_>>()
        .join("::"))
}
