use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};

#[derive(serde::Serialize, serde::Deserialize, Eq, Clone)]
pub enum Lifetime {
    /// The `'static` lifetime.
    Static,
    /// A named lifetime, e.g. `'a` in `&'a str`.
    /// It also include the "inferred" lifetime, which is represented as `'_`.
    Named(String),
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
            Lifetime::Named(_) | Lifetime::Elided => {
                // We don't care about the name of the lifetime, only that it is not static.
                state.write_u8(1);
            }
        }
    }
}

impl From<Option<String>> for Lifetime {
    fn from(s: Option<String>) -> Self {
        match s {
            Some(s) => {
                if &s == "'static" {
                    Lifetime::Static
                } else {
                    Lifetime::Named(s)
                }
            }
            None => Lifetime::Elided,
        }
    }
}

impl Lifetime {
    /// Returns `true` if this is the `'static` lifetime.
    pub fn is_static(&self) -> bool {
        match self {
            Lifetime::Named(_) | Lifetime::Elided => false,
            Lifetime::Static => true,
        }
    }

    /// Returns `true` if this lifetime was elided or inferred (`'_`).
    pub fn is_elided(&self) -> bool {
        match self {
            Lifetime::Named(n) if n == "_" => true,
            Lifetime::Elided => true,
            _ => false,
        }
    }
}

impl Debug for Lifetime {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Lifetime::Static => write!(f, "'static"),
            Lifetime::Named(name) => write!(f, "'{name}"),
            Lifetime::Elided => Ok(()),
        }
    }
}
