use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};

use crate::GenericLifetimeParameter;
use crate::named_lifetime::NamedLifetime;

#[derive(serde::Serialize, serde::Deserialize, Eq, Clone)]
pub enum Lifetime {
    /// The `'static` lifetime.
    Static,
    /// A named lifetime, e.g. `'a` in `&'a str`.
    Named(NamedLifetime),
    /// An inferred lifetime, i.e. `'_`.
    Inferred,
    /// A lifetime that is omitted from the source thanks to lifetime elision
    /// (see https://doc.rust-lang.org/nomicon/lifetime-elision.html).
    ///
    /// E.g. `&str`.
    Elided,
}

impl PartialEq for Lifetime {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Lifetime::Static, Lifetime::Static) => true,
            (Lifetime::Static, _) => false,
            (_, Lifetime::Static) => false,
            // We don't care about the name of the lifetime, only that it is not static.
            _ => true,
        }
    }
}

impl Hash for Lifetime {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Lifetime::Static => {
                state.write_u8(0);
            }
            Lifetime::Named(_) | Lifetime::Elided | Lifetime::Inferred => {
                // We don't care about the name of the lifetime, only that it is not static.
                state.write_u8(1);
            }
        }
    }
}

impl From<GenericLifetimeParameter> for Lifetime {
    fn from(l: GenericLifetimeParameter) -> Self {
        match l {
            GenericLifetimeParameter::Static => Lifetime::Static,
            GenericLifetimeParameter::Named(n) => Lifetime::Named(n),
            GenericLifetimeParameter::Inferred => Lifetime::Inferred,
        }
    }
}

impl From<Option<String>> for Lifetime {
    fn from(s: Option<String>) -> Self {
        match s {
            Some(s) => Lifetime::from_name(s),
            None => Lifetime::Elided,
        }
    }
}

impl Lifetime {
    /// Construct from a lifetime name, with or without the leading `'`.
    ///
    /// Routes `"_"` → `Inferred`, `"static"` → `Static`, everything else → `Named`.
    pub fn from_name(name: impl Into<String>) -> Self {
        let mut name = name.into();
        if let Some(stripped) = name.strip_prefix('\'') {
            name = stripped.to_owned();
        }
        match name.as_str() {
            "_" => Lifetime::Inferred,
            "static" => Lifetime::Static,
            _ => Lifetime::Named(NamedLifetime::new(name)),
        }
    }

    /// Returns `true` if this is the `'static` lifetime.
    pub fn is_static(&self) -> bool {
        match self {
            Lifetime::Named(_) | Lifetime::Elided | Lifetime::Inferred => false,
            Lifetime::Static => true,
        }
    }

    /// Returns `true` if this lifetime was elided or inferred (`'_`).
    pub fn is_elided(&self) -> bool {
        matches!(self, Lifetime::Elided | Lifetime::Inferred)
    }
}

impl Debug for Lifetime {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Lifetime::Static => write!(f, "'static"),
            Lifetime::Named(name) => write!(f, "'{name}"),
            Lifetime::Inferred => write!(f, "'_"),
            Lifetime::Elided => Ok(()),
        }
    }
}
