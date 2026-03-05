use std::fmt::{Debug, Formatter};

use crate::Type;

#[derive(serde::Serialize, serde::Deserialize, Eq, PartialEq, Hash, Clone)]
/// A Rust fixed-size array—e.g. `[u8; 4]`.
pub struct Array {
    /// The type of each element in the array.
    pub element_type: Box<Type>,
    /// The number of elements in the array.
    pub len: usize,
}

impl Debug for Array {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{:?}; {}]", self.element_type, self.len)
    }
}
