mod generic;
mod generic_argument;
pub(crate) mod generics_equivalence;
mod lifetime;
mod path_type;
pub(crate) mod render;
mod resolved_type;
mod scalar_primitive;
mod slice;
mod tuple;
mod type_reference;

pub use generic::Generic;
pub use generic_argument::{GenericArgument, GenericLifetimeParameter};
pub use lifetime::Lifetime;
pub use path_type::PathType;
pub use scalar_primitive::{ScalarPrimitive, UnknownPrimitive};
pub use slice::Slice;
pub use tuple::Tuple;
pub use type_reference::TypeReference;

/// A Rust type that has been fully resolved against rustdoc's JSON output.
#[derive(serde::Serialize, serde::Deserialize, Eq, PartialEq, Hash, Clone)]
pub enum ResolvedType {
    /// A named type identified by its fully-qualified path, e.g. `std::vec::Vec<u32>`.
    ResolvedPath(PathType),
    /// A reference type, e.g. `&str` or `&'a mut Vec<u8>`.
    Reference(TypeReference),
    /// A tuple type, e.g. `(u8, u16)` or the unit type `()`.
    Tuple(Tuple),
    /// A scalar primitive type, e.g. `u32`, `bool`, or `str`.
    ScalarPrimitive(ScalarPrimitive),
    /// A slice type, e.g. `[u8]`.
    Slice(Slice),
    /// An unassigned generic type parameter, e.g. `T`.
    Generic(Generic),
}
