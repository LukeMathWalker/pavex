pub(crate) use callable::{Callable, InvocationStyle};
pub(crate) use callable_path::{CallPath, InvalidCallPath};
pub(crate) use fq_path::{
    CallableItem, FQGenericArgument, FQPath, FQPathSegment, FQPathType, FQQualifiedSelf,
    ParseError, PathKind, ResolvedPathLifetime, UnknownPath,
};
pub use krate_name::{
    CrateNameResolutionError, UnknownCrate, UnknownDependency, dependency_name2package_id,
    krate2package_id,
};
pub(crate) use resolved_type::{
    Generic, GenericArgument, GenericLifetimeParameter, Lifetime, PathType, ResolvedType, Slice,
    Tuple, TypeReference,
};

mod callable;
mod callable_path;
mod fq_path;
mod krate_name;
mod resolved_type;
