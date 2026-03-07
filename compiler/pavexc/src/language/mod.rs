pub(crate) use callable::{Callable, CallableInput, InvocationStyle, ParameterName};
pub(crate) use callable_fq_path::{
    CallablePath, EnumVariantConstructorPath, FreeFunctionPath, InherentMethodPath,
    StructLiteralPath, TraitMethodPath,
};
pub(crate) use callable_path::{CallPath, InvalidCallPath};
pub(crate) use fq_path::{FQGenericArgument, FQPath, FQPathSegment, FQPathType, FQQualifiedSelf};
pub(crate) use fq_path_resolution::{
    CallableItem, PathKind, UnknownPath, find_rustdoc_callable_items, find_rustdoc_item_type,
    parse_fq_path, resolve_fq_path_type,
};
pub use krate_name::{
    CrateNameResolutionError, UnknownCrate, UnknownDependency, dependency_name2package_id,
    krate2package_id,
};
use pavex_bp_schema::CreatedAt;
pub(crate) use resolved_type::{
    Array, CanonicalType, Generic, GenericArgument, GenericLifetimeParameter, Lifetime, PathType,
    PathTypeExt, RawPointer, Slice, Tuple, Type, TypeReference, UnknownPrimitive,
};

mod callable;
mod callable_fq_path;
mod callable_path;
mod fq_path;
mod fq_path_resolution;
mod krate_name;
mod resolved_type;

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct RawIdentifiers {
    /// Information on the location where the component is defined.
    pub created_at: CreatedAt,
    /// An unambiguous path to the type/callable.
    pub import_path: String,
}
