pub(crate) use callable::{Callable, InvocationStyle};
pub(crate) use callable_path::{CallPath, InvalidCallPath};
pub(crate) use resolved_path::{
    ParseError, ResolvedPath, ResolvedPathGenericArgument, ResolvedPathLifetime,
    ResolvedPathQualifiedSelf, ResolvedPathSegment, UnknownPath,
};
pub(crate) use resolved_type::{
    Generic, GenericArgument, Lifetime, ResolvedPathType, ResolvedType, Slice, Tuple, TypeReference,
};

mod callable;
mod callable_path;
mod resolved_path;
mod resolved_type;

// E.g. `["std", "path", "PathBuf"]`.
pub type ImportPath = Vec<String>;
