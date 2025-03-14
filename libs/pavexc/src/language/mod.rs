pub(crate) use callable::{Callable, InvocationStyle};
pub(crate) use callable_path::{CallPath, InvalidCallPath};
pub use krate_name::{
    CrateNameResolutionError, UnknownCrateName, UnknownDependency, dependency_name2package_id,
    krate_name2package_id,
};
pub(crate) use resolved_path::{
    CallableItem, ParseError, PathKind, ResolvedPath, ResolvedPathGenericArgument,
    ResolvedPathLifetime, ResolvedPathQualifiedSelf, ResolvedPathSegment, ResolvedPathType,
    UnknownPath,
};
pub(crate) use resolved_type::{
    Generic, GenericArgument, GenericLifetimeParameter, Lifetime, PathType, ResolvedType, Slice,
    Tuple, TypeReference,
};

mod callable;
mod callable_path;
mod krate_name;
mod resolved_path;
mod resolved_type;
