use quote::{ToTokens, quote};

#[derive(darling::FromMeta, Debug, Clone, PartialEq, Eq)]
#[darling(rename_all = "snake_case")]
pub enum Lifecycle {
    Singleton,
    RequestScoped,
    Transient,
}

impl Lifecycle {
    pub fn as_str(&self) -> &'static str {
        match self {
            Lifecycle::Singleton => "singleton",
            Lifecycle::RequestScoped => "request_scoped",
            Lifecycle::Transient => "transient",
        }
    }
}

impl ToTokens for Lifecycle {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let s = self.as_str();
        tokens.extend(quote! { #s });
    }
}
