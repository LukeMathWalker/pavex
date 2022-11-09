use std::fmt::{Display, Formatter};

use syn::{ExprPath, GenericArgument, PathArguments, Type};

use pavex_builder::RawCallableIdentifiers;

/// A path that can be used in expression position (i.e. to refer to a function or a static method).
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub(crate) struct CallPath {
    pub has_leading_colon: bool,
    pub segments: Vec<CallPathSegment>,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub(crate) struct CallPathSegment {
    pub ident: syn::Ident,
    pub generic_arguments: Vec<CallPath>,
}

impl CallPath {
    pub fn parse(callable_identifiers: &RawCallableIdentifiers) -> Result<Self, InvalidCallPath> {
        let callable_path: ExprPath =
            syn::parse_str(callable_identifiers.raw_path()).map_err(|e| InvalidCallPath {
                raw_identifiers: callable_identifiers.to_owned(),
                parsing_error: e,
            })?;
        Self::parse_from_path(callable_path.path)
    }

    pub(crate) fn parse_from_path(path: syn::Path) -> Result<Self, InvalidCallPath> {
        let has_leading_colon = path.leading_colon.is_some();
        let mut segments = Vec::with_capacity(path.segments.len());
        for syn_segment in path.segments {
            let generic_arguments = match syn_segment.arguments {
                PathArguments::None => vec![],
                PathArguments::AngleBracketed(syn_arguments) => {
                    let mut arguments = Vec::with_capacity(syn_arguments.args.len());
                    for syn_argument in syn_arguments.args {
                        let argument = match syn_argument {
                            GenericArgument::Type(p) => match p {
                                Type::Path(p) => Self::parse_from_path(p.path)?,
                                _ => unreachable!(),
                            },
                            GenericArgument::Lifetime(_)
                            | GenericArgument::Binding(_)
                            | GenericArgument::Constraint(_)
                            | GenericArgument::Const(_) => todo!(
                                "We can only handle generic type parameters for the time being."
                            ),
                        };
                        arguments.push(argument)
                    }
                    arguments
                }
                PathArguments::Parenthesized(_) => {
                    todo!("We do not handle paranthesized generic parameters")
                }
            };
            let segment = CallPathSegment {
                ident: syn_segment.ident,
                generic_arguments,
            };
            segments.push(segment)
        }
        Ok(Self {
            has_leading_colon,
            segments,
        })
    }

    /// Return the first segment in the path.
    ///
    /// E.g. `my_crate::my_module::MyType` will return `my_crate` while `my_module::MyType` will
    /// return `my_module`.
    pub fn leading_path_segment(&self) -> &proc_macro2::Ident {
        // This unwrap never fails thanks to the validation done in `parse`
        &self.segments.first().unwrap().ident
    }
}

impl Display for CallPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let leading_colon = if self.has_leading_colon { "::" } else { "" };
        write!(f, "{}", leading_colon)?;
        let last_segment_index = self.segments.len().saturating_sub(1);
        for (i, segment) in self.segments.iter().enumerate() {
            write!(f, "{}", segment)?;
            if i != last_segment_index {
                write!(f, "::")?;
            }
        }
        Ok(())
    }
}

impl Display for CallPathSegment {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.ident)?;
        let last_argument_index = self.generic_arguments.len().saturating_sub(1);
        for (j, argument) in self.generic_arguments.iter().enumerate() {
            write!(f, "{}", argument)?;
            if j != last_argument_index {
                write!(f, ", ")?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub struct InvalidCallPath {
    pub(crate) raw_identifiers: RawCallableIdentifiers,
    #[source]
    pub(crate) parsing_error: syn::Error,
}

impl Display for InvalidCallPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let path = self.raw_identifiers.raw_path();
        write!(f, "`{path}` is not a valid import path.")
    }
}
