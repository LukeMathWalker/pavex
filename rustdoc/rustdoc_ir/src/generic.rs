use std::fmt::{Debug, Formatter};

#[derive(serde::Serialize, serde::Deserialize, Eq, PartialEq, Hash, Clone)]
/// An unassigned generic parameter—e.g. `T` in `fn foo<T>(t: T)`.
pub struct Generic {
    /// The name of the generic parameter, e.g. `"T"`.
    pub name: String,
}

impl Debug for Generic {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}
