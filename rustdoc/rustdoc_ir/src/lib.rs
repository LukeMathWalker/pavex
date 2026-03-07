mod array;
mod generic;
mod generic_argument;
pub(crate) mod generics_equivalence;
mod lifetime;
mod named_lifetime;
mod path_type;
mod raw_pointer;
pub(crate) mod render;
mod scalar_primitive;
mod slice;
mod tuple;
mod type_;
mod type_reference;

pub use array::Array;
pub use generic::Generic;
pub use generic_argument::{GenericArgument, GenericLifetimeParameter};
pub use named_lifetime::NamedLifetime;
pub use lifetime::Lifetime;
pub use path_type::PathType;
pub use raw_pointer::RawPointer;
pub use scalar_primitive::{ScalarPrimitive, UnknownPrimitive};
pub use slice::Slice;
pub use tuple::Tuple;
pub use type_::CanonicalType;
pub use type_reference::TypeReference;

/// A Rust type.
#[derive(serde::Serialize, serde::Deserialize, Eq, PartialEq, Hash, Clone)]
pub enum Type {
    /// A named type identified by its fully-qualified path, e.g. `std::vec::Vec<u32>`.
    Path(PathType),
    /// A reference type, e.g. `&str` or `&'a mut Vec<u8>`.
    Reference(TypeReference),
    /// A tuple type, e.g. `(u8, u16)` or the unit type `()`.
    Tuple(Tuple),
    /// A scalar primitive type, e.g. `u32`, `bool`, or `str`.
    ScalarPrimitive(ScalarPrimitive),
    /// A slice type, e.g. `[u8]`.
    Slice(Slice),
    /// A fixed-size array type, e.g. `[u8; 4]`.
    Array(Array),
    /// A raw pointer type, e.g. `*const u8` or `*mut u8`.
    RawPointer(RawPointer),
    /// An unassigned generic type parameter, e.g. `T`.
    Generic(Generic),
}
