use std::fmt::{Debug, Display, Formatter};

use bimap::BiHashMap;
use guppy::PackageId;

use crate::Type;
use crate::named_lifetime::NamedLifetime;
use crate::render::{LifetimeStyle, PathStyle, RenderConfig};

/// A single generic argument supplied to a generic type or function.
#[derive(serde::Serialize, serde::Deserialize, Eq, PartialEq, Hash, Clone)]
pub enum GenericArgument {
    /// A generic type parameter, e.g. `u32` in `Vec<u32>` or `T` in `HashSet<T>`.
    TypeParameter(Type),
    /// A lifetime parameter, e.g. `'a` in `Cow<'a, str>`.
    Lifetime(GenericLifetimeParameter),
    /// A const generic argument with a concrete evaluated value, e.g. `8` in `Size<8>`.
    Const(ConstGenericArgument),
}

/// A const generic argument with a concrete evaluated value, e.g. `8` in `Size<8>`.
#[derive(serde::Serialize, serde::Deserialize, Eq, PartialEq, Hash, Clone)]
pub struct ConstGenericArgument {
    /// The evaluated value as a string, e.g. "8", "true", "'a'".
    pub value: String,
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

impl GenericArgument {
    /// Render this generic argument preserving named lifetimes as-is.
    pub fn render_type_into(&self, id2name: &BiHashMap<PackageId, String>, buffer: &mut String) {
        let config = RenderConfig {
            path: PathStyle::CrateLookup(id2name),
            lifetime: LifetimeStyle::Preserve,
        };
        self.render_into(&config, buffer);
    }

    /// Render this generic argument, replacing named lifetimes with `'_`.
    pub fn render_with_inferred_lifetimes_into(
        &self,
        id2name: &BiHashMap<PackageId, String>,
        buffer: &mut String,
    ) {
        let config = RenderConfig {
            path: PathStyle::CrateLookup(id2name),
            lifetime: LifetimeStyle::Erase,
        };
        self.render_into(&config, buffer);
    }

    /// Render this generic argument for error messages.
    pub fn display_for_error_into<W: std::fmt::Write>(&self, buffer: &mut W) {
        let config = RenderConfig {
            path: PathStyle::Direct,
            lifetime: LifetimeStyle::Preserve,
        };
        self.render_into(&config, buffer);
    }

    /// Render this generic argument into `buffer` according to `config`.
    ///
    /// This is the single implementation behind [`GenericArgument::render_type_into`],
    /// [`GenericArgument::render_with_inferred_lifetimes_into`], and
    /// [`GenericArgument::display_for_error_into`].
    pub(crate) fn render_into<W: std::fmt::Write>(
        &self,
        config: &RenderConfig<'_>,
        buffer: &mut W,
    ) {
        match self {
            GenericArgument::TypeParameter(t) => {
                t.render_into(config, buffer);
            }
            GenericArgument::Lifetime(l) => match config.lifetime {
                LifetimeStyle::Preserve => {
                    write!(buffer, "{l}").unwrap();
                }
                LifetimeStyle::Erase => match l {
                    GenericLifetimeParameter::Static => {
                        write!(buffer, "'static").unwrap();
                    }
                    GenericLifetimeParameter::Named(_) | GenericLifetimeParameter::Inferred => {
                        write!(buffer, "'_").unwrap();
                    }
                },
            },
            GenericArgument::Const(c) => {
                write!(buffer, "{}", c.value).unwrap();
            }
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
            GenericArgument::Const(c) => write!(f, "{}", c.value),
        }
    }
}

impl Debug for GenericArgument {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            GenericArgument::TypeParameter(r) => write!(f, "{r:?}"),
            GenericArgument::Lifetime(l) => write!(f, "{l:?}"),
            GenericArgument::Const(c) => write!(f, "{}", c.value),
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
