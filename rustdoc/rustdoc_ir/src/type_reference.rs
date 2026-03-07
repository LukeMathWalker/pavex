use std::fmt::{Debug, Formatter};

use crate::{Lifetime, Type};

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash, Clone)]
/// A Rust reference—e.g. `&mut u32` or `&'static mut Vec<u8>`.
pub struct TypeReference {
    /// `true` if this is a mutable reference (`&mut T`).
    pub is_mutable: bool,
    /// The lifetime of this reference.
    pub lifetime: Lifetime,
    /// The type being referenced.
    pub inner: Box<Type>,
}

impl Debug for TypeReference {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "&")?;
        if !self.lifetime.is_elided() {
            write!(f, "{:?} ", self.lifetime)?;
        }

        if self.is_mutable {
            write!(f, "mut ")?;
        }
        write!(f, "{:?}", self.inner)?;
        Ok(())
    }
}
