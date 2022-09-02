use std::fmt::{Display, Formatter};

use quote::quote;
use syn::ExprPath;

use pavex_builder::RawCallableIdentifiers;

/// A path that can be used in expression position (i.e. to refer to a function or a static method).
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub(crate) struct CallPath(pub ExprPath);

impl AsRef<ExprPath> for CallPath {
    fn as_ref(&self) -> &ExprPath {
        &self.0
    }
}

impl CallPath {
    pub fn parse(callable_identifiers: &RawCallableIdentifiers) -> Result<Self, InvalidCallPath> {
        let callable_path: ExprPath =
            syn::parse_str(callable_identifiers.raw_path()).map_err(|e| InvalidCallPath {
                raw_identifiers: callable_identifiers.to_owned(),
                parsing_error: e,
            })?;
        Ok(Self(callable_path))
    }

    /// Return the first segment in the path.
    ///
    /// E.g. `my_crate::my_module::MyType` will return `my_crate` while `my_module::MyType` will
    /// return `my_module`.
    pub fn leading_path_segment(&self) -> &proc_macro2::Ident {
        // This unwrap never fails thanks to the validation done in `parse`
        &self.0.path.segments.first().unwrap().ident
    }
}

impl std::fmt::Display for CallPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let path = &self.0;
        let s = quote! { #path }.to_string();
        write!(
            f,
            "{}",
            s.replace(" :: ", "::")
                .replace("< ", "<")
                .replace(" >", ">")
        )
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) struct InvalidCallPath {
    pub raw_identifiers: RawCallableIdentifiers,
    #[source]
    pub parsing_error: syn::Error,
}

impl Display for InvalidCallPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let path = self.raw_identifiers.raw_path();
        write!(f, "`{path}` is not a valid import path.")
    }
}
