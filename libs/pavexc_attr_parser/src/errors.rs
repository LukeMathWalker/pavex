use itertools::Itertools;

#[derive(Debug, thiserror::Error)]
/// Failure modes of [`parse`](crate::parse).
pub enum AttributeParserError {
    #[error(transparent)]
    UnknownPavexAttribute(#[from] UnknownPavexAttribute),
    #[error(transparent)]
    InvalidAttributeParams(#[from] InvalidAttributeParams),
    #[error("Multiple `pavex::diagnostic::*` attributes on the same item")]
    MultiplePavexAttributes,
}

#[derive(Debug)]
pub struct UnknownPavexAttribute {
    pub path: syn::Path,
}

impl std::fmt::Display for UnknownPavexAttribute {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let p = self
            .path
            .segments
            .iter()
            .map(|s| format!("{}", s.ident))
            .join("::");
        write!(f, "Unknown Pavex attribute: `#[{}(...)]`", p)
    }
}

impl std::error::Error for UnknownPavexAttribute {}

#[derive(Debug, thiserror::Error)]
#[error("{0} for `{1}` attribute")]
pub struct InvalidAttributeParams(darling::Error, &'static str);

impl InvalidAttributeParams {
    pub fn constructor(e: darling::Error) -> Self {
        Self(e, "pavex::diagnostic::constructor")
    }
}
