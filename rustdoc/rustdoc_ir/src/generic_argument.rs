use std::fmt::{Debug, Display, Formatter};
use std::hash::{Hash, Hasher};

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
#[derive(serde::Serialize, serde::Deserialize, Eq, Clone)]
pub enum GenericLifetimeParameter {
    /// A named (non-static) lifetime, e.g. `'a`.
    Named(String),
    /// The `'static` lifetime.
    Static,
}

impl Display for GenericLifetimeParameter {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            GenericLifetimeParameter::Named(s) => write!(f, "'{s}"),
            GenericLifetimeParameter::Static => write!(f, "'static"),
        }
    }
}

impl PartialEq for GenericLifetimeParameter {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (GenericLifetimeParameter::Static, GenericLifetimeParameter::Static) => true,
            (GenericLifetimeParameter::Static, _) => false,
            // We don't care about the name of the lifetime, only that it is not static.
            _ => true,
        }
    }
}

impl Hash for GenericLifetimeParameter {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            GenericLifetimeParameter::Static => {
                state.write_u8(0);
            }
            GenericLifetimeParameter::Named(_) => {
                // We don't care about the name of the lifetime, only that it is not static.
                state.write_u8(1);
            }
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
        }
    }
}
