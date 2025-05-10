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

#[derive(Debug, thiserror::Error)]
#[error("Unknown Pavex attribute: `#[{path}(...)]`")]
pub struct UnknownPavexAttribute {
    pub path: String,
}

impl UnknownPavexAttribute {
    pub fn new(path: &syn::Path) -> Self {
        let path = path
            .segments
            .iter()
            .map(|s| format!("{}", s.ident))
            .join("::");
        Self { path }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("{0} for `{1}` attribute")]
pub struct InvalidAttributeParams(darling::Error, &'static str);

impl InvalidAttributeParams {
    pub fn constructor(e: darling::Error) -> Self {
        Self(e, "pavex::diagnostic::constructor")
    }

    pub fn config(e: darling::Error) -> Self {
        Self(e, "pavex::diagnostic::config")
    }

    pub fn wrap(e: darling::Error) -> Self {
        Self(e, "pavex::diagnostic::wrap")
    }
}
