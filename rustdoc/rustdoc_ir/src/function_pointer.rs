use std::fmt::{Debug, Formatter};

use crate::Type;

/// A Rust function pointer type—e.g. `fn(u32) -> u8` or `fn()`.
#[derive(serde::Serialize, serde::Deserialize, Eq, PartialEq, Hash, Clone)]
pub struct FunctionPointer {
    /// The input parameter types.
    pub inputs: Vec<Type>,
    /// The return type. `None` means the unit type `()`.
    pub output: Option<Box<Type>>,
}

impl Debug for FunctionPointer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "fn(")?;
        for (i, input) in self.inputs.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{:?}", input)?;
        }
        write!(f, ")")?;
        if let Some(output) = &self.output {
            write!(f, " -> {:?}", output)?;
        }
        Ok(())
    }
}
