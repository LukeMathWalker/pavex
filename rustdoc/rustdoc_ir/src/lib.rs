mod array;
mod callable;
mod callable_path;
mod canonical_path_resolver;
pub mod function_pointer;
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
pub use callable::{
    Callable, CallableInput, EnumVariantInit, FnHeader, FreeFunction, InherentMethod,
    RustIdentifier, StructLiteralInit, TraitMethod,
};
pub use callable_path::{
    EnumVariantConstructorPath, FreeFunctionPath, InherentMethodPath, StructLiteralPath,
    TraitMethodPath,
};
pub use canonical_path_resolver::{CanonicalPathResolver, NoOpResolver};
pub use function_pointer::{FunctionPointer, FunctionPointerInput};
pub use generic::Generic;
pub use generic_argument::{ConstGenericArgument, GenericArgument, GenericLifetimeParameter};
pub use lifetime::Lifetime;
pub use named_lifetime::NamedLifetime;
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
    /// A function pointer type, e.g. `fn(u32) -> u8`.
    FunctionPointer(FunctionPointer),
    /// An unassigned generic type parameter, e.g. `T`.
    Generic(Generic),
    /// A type alias, preserving the alias identity rather than resolving through.
    /// Contains the alias's path (package, base_type, generic arguments).
    TypeAlias(PathType),
}
