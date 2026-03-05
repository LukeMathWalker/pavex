use std::fmt::{Debug, Formatter};

use crate::Type;

#[derive(serde::Serialize, serde::Deserialize, Eq, PartialEq, Hash, Clone)]
/// A Rust slice—e.g. `[u16]`.
pub struct Slice {
    /// The type of each element in the slice.
    pub element_type: Box<Type>,
}

impl Debug for Slice {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{:?}]", self.element_type)
    }
}
