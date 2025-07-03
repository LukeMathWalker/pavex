pub(crate) use callable::{Callable, InvocationStyle};
pub(crate) use callable_path::{CallPath, InvalidCallPath};
pub(crate) use fq_path::{
    CallableItem, FQGenericArgument, FQPath, FQPathSegment, FQPathType, FQQualifiedSelf, PathKind,
    ResolvedPathLifetime, UnknownPath,
};
pub use krate_name::{
    CrateNameResolutionError, UnknownCrate, UnknownDependency, dependency_name2package_id,
    krate2package_id,
};
use pavex_bp_schema::CreatedAt;
pub(crate) use resolved_type::{
    Generic, GenericArgument, GenericLifetimeParameter, Lifetime, PathType, ResolvedType, Slice,
    Tuple, TypeReference,
};

mod callable;
mod callable_path;
mod fq_path;
mod krate_name;
mod resolved_type;

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct RawIdentifiers {
    /// Information on the location where the component is defined.
    pub created_at: CreatedAt,
    /// An unambiguous path to the type/callable.
    pub import_path: String,
}
