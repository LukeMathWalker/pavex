use darling::util::Flag;
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

pub struct CloningStrategyFlags {
    pub clone_if_necessary: Flag,
    pub never_clone: Flag,
}

impl TryFrom<CloningStrategyFlags> for Option<CloningStrategy> {
    type Error = darling::Error;

    fn try_from(flags: CloningStrategyFlags) -> Result<Self, Self::Error> {
        match (
            flags.never_clone.is_present(),
            flags.clone_if_necessary.is_present(),
        ) {
            (true, true) => Err(darling::Error::custom(
                "You can only specify *one* of `never_clone` and `clone_if_necessary`.",
            )),
            (true, false) => Ok(Some(CloningStrategy::NeverClone)),
            (false, true) => Ok(Some(CloningStrategy::CloneIfNecessary)),
            (false, false) => Ok(None),
        }
    }
}
