use darling::util::Flag;
use quote::{ToTokens, quote};

#[derive(darling::FromMeta, Debug, Clone, PartialEq, Eq)]
#[darling(rename_all = "snake_case")]
pub enum CloningPolicy {
    CloneIfNecessary,
    NeverClone,
}
impl CloningPolicy {
    pub fn as_str(&self) -> &'static str {
        match self {
            CloningPolicy::CloneIfNecessary => "clone_if_necessary",
            CloningPolicy::NeverClone => "never_clone",
        }
    }
}

impl ToTokens for CloningPolicy {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let s = self.as_str();
        tokens.extend(quote! { #s });
    }
}

pub struct CloningPolicyFlags {
    pub clone_if_necessary: Flag,
    pub never_clone: Flag,
}

impl TryFrom<CloningPolicyFlags> for Option<CloningPolicy> {
    type Error = darling::Error;

    fn try_from(flags: CloningPolicyFlags) -> Result<Self, Self::Error> {
        match (
            flags.never_clone.is_present(),
            flags.clone_if_necessary.is_present(),
        ) {
            (true, true) => Err(darling::Error::custom(
                "You can only specify *one* of `never_clone` and `clone_if_necessary`.",
            )),
            (true, false) => Ok(Some(CloningPolicy::NeverClone)),
            (false, true) => Ok(Some(CloningPolicy::CloneIfNecessary)),
            (false, false) => Ok(None),
        }
    }
}
