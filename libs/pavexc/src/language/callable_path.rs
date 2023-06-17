use std::fmt::{Display, Formatter};

use syn::{ExprPath, GenericArgument, PathArguments, Type};

use pavex::blueprint::reflection::RawCallableIdentifiers;

/// A path that can be used in expression position (i.e. to refer to a function or a static method).
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub(crate) struct CallPath {
    pub has_leading_colon: bool,
    pub qualified_self: Option<CallPathQualifiedSelf>,
    pub segments: Vec<CallPathSegment>,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub(crate) struct CallPathQualifiedSelf {
    pub position: usize,
    pub type_: CallPathType,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub(crate) enum CallPathType {
    ResolvedPath(CallPathResolvedPathType),
    Reference(CallPathReference),
    Tuple(CallPathTuple),
    Slice(CallPathSlice),
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub(crate) struct CallPathResolvedPathType {
    pub path: Box<CallPath>,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub(crate) struct CallPathSlice {
    pub element_type: Box<CallPathType>,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub(crate) struct CallPathReference {
    pub is_mutable: bool,
    pub is_static: bool,
    pub inner: Box<CallPathType>,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub(crate) struct CallPathTuple {
    pub elements: Vec<CallPathType>,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub(crate) struct CallPathSegment {
    pub ident: syn::Ident,
    pub generic_arguments: Vec<CallPathGenericArgument>,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub(crate) enum CallPathGenericArgument {
    Type(CallPathType),
    Lifetime(CallPathLifetime),
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub(crate) enum CallPathLifetime {
    Static,
    Named(String),
}

impl CallPathLifetime {
    fn new(l: String) -> Self {
        match l.trim_start_matches('\'') {
            "static" => Self::Static,
            other => Self::Named(other.to_owned()),
        }
    }
}

impl CallPath {
    pub fn parse(callable_identifiers: &RawCallableIdentifiers) -> Result<Self, InvalidCallPath> {
        let callable_path: ExprPath =
            syn::parse_str(callable_identifiers.raw_path()).map_err(|e| InvalidCallPath {
                raw_identifiers: callable_identifiers.to_owned(),
                parsing_error: e,
            })?;
        Self::parse_from_path(callable_path.path, callable_path.qself)
    }

    fn parse_qself(qself: syn::QSelf) -> Result<CallPathQualifiedSelf, InvalidCallPath> {
        Ok(CallPathQualifiedSelf {
            position: qself.position,
            type_: Self::parse_type(*qself.ty)?,
        })
    }

    fn parse_type(type_: Type) -> Result<CallPathType, InvalidCallPath> {
        match type_ {
            Type::Path(p) => {
                let call_path = Self::parse_from_path(p.path, p.qself)?;
                Ok(CallPathType::ResolvedPath(CallPathResolvedPathType {
                    path: Box::new(call_path),
                }))
            }
            Type::Reference(r) => {
                let is_mutable = r.mutability.is_some();
                let inner = Box::new(Self::parse_type(*r.elem)?);
                Ok(CallPathType::Reference(CallPathReference {
                    is_mutable,
                    is_static: r
                        .lifetime
                        .map(|l| l.ident.to_string().as_str() == "'static")
                        .unwrap_or(false),
                    inner,
                }))
            }
            Type::Tuple(t) => {
                let mut elements = Vec::with_capacity(t.elems.len());
                for element in t.elems {
                    elements.push(Self::parse_type(element)?)
                }
                Ok(CallPathType::Tuple(CallPathTuple { elements }))
            }
            Type::Slice(s) => {
                let element_type = Box::new(Self::parse_type(s.elem.as_ref().to_owned())?);
                Ok(CallPathType::Slice(CallPathSlice { element_type }))
            }
            _ => todo!("We don't handle {:?} as a type yet", type_),
        }
    }

    pub(crate) fn parse_from_path(
        path: syn::Path,
        qualified_self: Option<syn::QSelf>,
    ) -> Result<Self, InvalidCallPath> {
        let has_leading_colon = path.leading_colon.is_some();
        let mut segments = Vec::with_capacity(path.segments.len());
        for syn_segment in path.segments {
            let generic_arguments = match syn_segment.arguments {
                PathArguments::None => vec![],
                PathArguments::AngleBracketed(syn_arguments) => {
                    let mut arguments = Vec::with_capacity(syn_arguments.args.len());
                    for syn_argument in syn_arguments.args {
                        let argument = match syn_argument {
                            GenericArgument::Type(t) => {
                                CallPathGenericArgument::Type(Self::parse_type(t)?)
                            }
                            GenericArgument::Lifetime(l) => {
                                CallPathGenericArgument::Lifetime(CallPathLifetime::new(l.ident.to_string()))
                            }
                            GenericArgument::AssocType(_)
                            | GenericArgument::AssocConst(_)
                            | GenericArgument::Constraint(_)
                            | GenericArgument::Const(_)
                            | _ => todo!(
                                "We can only handle concrete types and lifetimes as generic parameters for the time being."
                            ),
                        };
                        arguments.push(argument)
                    }
                    arguments
                }
                PathArguments::Parenthesized(_) => {
                    todo!("We don't handle paranthesized generic parameters")
                }
            };
            let segment = CallPathSegment {
                ident: syn_segment.ident,
                generic_arguments,
            };
            segments.push(segment)
        }

        let qualified_self = if let Some(qself) = qualified_self {
            Some(Self::parse_qself(qself)?)
        } else {
            None
        };
        Ok(Self {
            has_leading_colon,
            qualified_self,
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

impl Display for CallPathType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CallPathType::ResolvedPath(p) => {
                write!(f, "{}", p)?;
            }
            CallPathType::Reference(r) => {
                write!(f, "{}", r)?;
            }
            CallPathType::Tuple(t) => {
                write!(f, "{}", t)?;
            }
            CallPathType::Slice(s) => {
                write!(f, "{}", s)?;
            }
        }
        Ok(())
    }
}

impl Display for CallPathSlice {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}]", self.element_type)
    }
}

impl Display for CallPathTuple {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "(")?;
        let last_argument_index = self.elements.len().saturating_sub(1);
        for (i, element) in self.elements.iter().enumerate() {
            write!(f, "{}", element)?;
            if i != last_argument_index {
                write!(f, ", ")?;
            }
        }
        write!(f, ")")
    }
}

impl Display for CallPathReference {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "&{}{}",
            if self.is_mutable { "mut " } else { "" },
            self.inner
        )
    }
}

impl Display for CallPathResolvedPathType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.path)
    }
}

impl Display for CallPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut qself_closing_wedge_index = None;
        if let Some(qself) = &self.qualified_self {
            write!(f, "<{} as ", qself.type_)?;
            qself_closing_wedge_index = Some(qself.position);
        }
        if self.has_leading_colon {
            write!(f, "::")?;
        }
        let last_segment_index = self.segments.len().saturating_sub(1);
        for (i, segment) in self.segments.iter().enumerate() {
            write!(f, "{segment}")?;
            if Some(i) == qself_closing_wedge_index {
                write!(f, ">")?;
            }
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
            write!(f, "{argument}")?;
            if j != last_argument_index {
                write!(f, ", ")?;
            }
        }
        Ok(())
    }
}

impl Display for CallPathGenericArgument {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CallPathGenericArgument::Type(t) => {
                write!(f, "{}", t)?;
            }
            CallPathGenericArgument::Lifetime(l) => {
                write!(f, "{}", l)?;
            }
        }
        Ok(())
    }
}

impl Display for CallPathLifetime {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CallPathLifetime::Static => write!(f, "'static"),
            CallPathLifetime::Named(name) => write!(f, "'{}", name),
        }
    }
}

#[derive(Debug, thiserror::Error, Clone)]
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
