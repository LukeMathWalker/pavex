use std::fmt::{Debug, Formatter};

use crate::Type;

#[derive(serde::Serialize, serde::Deserialize, Eq, PartialEq, Hash, Clone)]
/// A Rust tuple—e.g. `(u8, u16, u32)`.
pub struct Tuple {
    /// The types of each element in the tuple. An empty vector represents the unit type `()`.
    pub elements: Vec<Type>,
}

impl Debug for Tuple {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "(")?;
        let mut elements = self.elements.iter().peekable();
        while let Some(element) = elements.next() {
            write!(f, "{element:?}")?;
            if elements.peek().is_some() {
                write!(f, ", ")?;
            }
        }
        write!(f, ")")?;
        Ok(())
    }
}
