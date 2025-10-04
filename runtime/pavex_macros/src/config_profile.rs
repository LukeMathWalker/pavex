//! A derive macro for implementing the `ConfigProfile` trait for C-style enums.
use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Fields, Lit, parse_macro_input};

pub(super) fn derive_config_profile(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let unsupported_error_msg = "An implementation of the `ConfigProfile` trait can only be derived for enums with unit variants (no fields). \
        Consider implementing the trait manually.";

    let Data::Enum(data_enum) = &input.data else {
        return syn::Error::new_spanned(name, unsupported_error_msg)
            .to_compile_error()
            .into();
    };

    let mut variant_mappings = Vec::new();
    let mut match_arms_from_str = Vec::new();
    let mut valid_profiles = Vec::new();

    for variant in &data_enum.variants {
        if !matches!(variant.fields, Fields::Unit) {
            return syn::Error::new_spanned(&variant.ident, unsupported_error_msg)
                .to_compile_error()
                .into();
        }

        let variant_name = &variant.ident;
        let mut profile_name = variant_name.to_string().to_case(Case::Snake);

        for attr in &variant.attrs {
            if !attr.meta.path().is_ident("px") {
                continue;
            }
            let error_msg = "Invalid `px` attribute. Expected `#[px(profile = \"name\")]`";
            let Ok(args) = attr.parse_args::<syn::MetaNameValue>() else {
                return syn::Error::new_spanned(attr, error_msg)
                    .to_compile_error()
                    .into();
            };
            if !args.path.is_ident("profile") {
                return syn::Error::new_spanned(&args.path, error_msg)
                    .to_compile_error()
                    .into();
            }
            let syn::Expr::Lit(syn::ExprLit {
                lit: Lit::Str(lit_str),
                ..
            }) = &args.value
            else {
                return syn::Error::new_spanned(&args.value, error_msg)
                    .to_compile_error()
                    .into();
            };
            profile_name = lit_str.value();
            let Some(first) = profile_name.chars().next() else {
                return syn::Error::new_spanned(&args.value, "Profile name cannot be empty.")
                    .to_compile_error()
                    .into();
            };
            if !first.is_alphabetic()
                || !profile_name
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '_')
            {
                return syn::Error::new_spanned(&args.value, "Profile name must start with a letter and only contain letters, numbers, or underscores.")
                    .to_compile_error()
                    .into();
            }
        }

        valid_profiles.push(profile_name.clone());
        variant_mappings.push(quote! {
            #name::#variant_name => #profile_name,
        });
        match_arms_from_str.push(quote! {
            #profile_name => Ok(#name::#variant_name),
        });
    }

    let error_name = format_ident!("{}ParseError", name);
    let valid_profiles_str = valid_profiles
        .iter()
        .map(|s| format!("`{s}`"))
        .collect::<Vec<_>>()
        .join(", ");

    let expanded = quote! {
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct #error_name(String);

        impl std::fmt::Display for #error_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "Invalid profile: `{}`. Valid options are: {}", self.0, #valid_profiles_str)
            }
        }

        impl std::error::Error for #error_name {}

        #[automatically_derived]
        impl std::str::FromStr for #name {
            type Err = #error_name;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    #(#match_arms_from_str)*
                    _ => Err(#error_name(s.to_string())),
                }
            }
        }

        #[automatically_derived]
        impl std::convert::AsRef<str> for #name {
            fn as_ref(&self) -> &str {
                match self {
                    #(#variant_mappings)*
                }
            }
        }

        #[automatically_derived]
        impl pavex::config::ConfigProfile for #name {}
    };

    TokenStream::from(expanded)
}
