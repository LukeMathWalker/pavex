use std::fmt::{Debug, Formatter};

use crate::Type;

#[derive(serde::Serialize, serde::Deserialize, Eq, PartialEq, Hash, Clone)]
/// A Rust raw pointer—e.g. `*const u8` or `*mut Vec<u8>`.
pub struct RawPointer {
    /// `true` if this is a `*mut T` pointer, `false` if `*const T`.
    pub is_mutable: bool,
    /// The type being pointed to.
    pub inner: Box<Type>,
}

impl Debug for RawPointer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.is_mutable {
            write!(f, "*mut ")?;
        } else {
            write!(f, "*const ")?;
        }
        write!(f, "{:?}", self.inner)
    }
}
