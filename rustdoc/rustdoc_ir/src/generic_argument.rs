use std::fmt::{Debug, Display, Formatter};

use crate::named_lifetime::NamedLifetime;
use crate::Type;

/// A single generic argument supplied to a generic type or function.
#[derive(serde::Serialize, serde::Deserialize, Eq, PartialEq, Hash, Clone)]
pub enum GenericArgument {
    /// A generic type parameter, e.g. `u32` in `Vec<u32>` or `T` in `HashSet<T>`.
    TypeParameter(Type),
    /// A lifetime parameter, e.g. `'a` in `Cow<'a, str>`.
    Lifetime(GenericLifetimeParameter),
}

/// A lifetime used as a generic argument—e.g. `'a` in `Cow<'a, str>`.
#[derive(serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash, Clone)]
pub enum GenericLifetimeParameter {
    /// A named (non-static) lifetime, e.g. `'a`.
    Named(NamedLifetime),
    /// The `'static` lifetime.
    Static,
    /// An inferred lifetime, i.e. `'_`.
    Inferred,
}

impl GenericLifetimeParameter {
    /// Construct from a lifetime name, with or without the leading `'`.
    ///
    /// Routes `"_"` → `Inferred`, `"static"` → `Static`, everything else → `Named`.
    pub fn from_name(name: impl Into<String>) -> Self {
        let mut name = name.into();
        if let Some(stripped) = name.strip_prefix('\'') {
            name = stripped.to_owned();
        }
        match name.as_str() {
            "_" => GenericLifetimeParameter::Inferred,
            "static" => GenericLifetimeParameter::Static,
            _ => GenericLifetimeParameter::Named(NamedLifetime::new(name)),
        }
    }
}

impl Display for GenericLifetimeParameter {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            GenericLifetimeParameter::Named(l) => write!(f, "'{l}"),
            GenericLifetimeParameter::Static => write!(f, "'static"),
            GenericLifetimeParameter::Inferred => write!(f, "'_"),
        }
    }
}

impl Display for GenericArgument {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            GenericArgument::TypeParameter(t) => write!(f, "{t}"),
            GenericArgument::Lifetime(l) => write!(f, "{l}"),
        }
    }
}

impl Debug for GenericArgument {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            GenericArgument::TypeParameter(r) => write!(f, "{r:?}"),
            GenericArgument::Lifetime(l) => write!(f, "{l:?}"),
        }
    }
}

impl Debug for GenericLifetimeParameter {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            GenericLifetimeParameter::Named(name) => write!(f, "'{name}"),
            GenericLifetimeParameter::Static => write!(f, "'static"),
            GenericLifetimeParameter::Inferred => write!(f, "'_"),
        }
    }
}
