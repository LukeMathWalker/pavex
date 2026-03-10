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
    Callable, CallableInput, CanonicalType, EnumVariantConstructorPath, EnumVariantInit, FnHeader,
    FreeFunction, FreeFunctionPath, Generic, GenericArgument, GenericLifetimeParameter,
    InherentMethod, InherentMethodPath, Lifetime, PathType, PathTypeExt, RawPointer,
    RustIdentifier, StructLiteralInit, TraitMethod, TraitMethodPath, Tuple, Type, TypeReference,
};

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
