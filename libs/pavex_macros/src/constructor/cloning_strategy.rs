use quote::{ToTokens, quote};

#[derive(darling::FromMeta, Debug, Clone, PartialEq, Eq)]
#[darling(rename_all = "snake_case")]
pub enum CloningStrategy {
    CloneIfNecessary,
    NeverClone,
}
impl CloningStrategy {
    pub fn as_str(&self) -> &'static str {
        match self {
            CloningStrategy::CloneIfNecessary => "clone_if_necessary",
            CloningStrategy::NeverClone => "never_clone",
        }
    }
}

impl ToTokens for CloningStrategy {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let s = self.as_str();
        tokens.extend(quote! { #s });
    }
}
