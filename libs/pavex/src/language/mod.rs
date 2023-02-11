pub(crate) use callable::{Callable, InvocationStyle};
pub(crate) use callable_path::{CallPath, InvalidCallPath};
pub(crate) use resolved_path::{
    ParseError, ResolvedPath, ResolvedPathQualifiedSelf, ResolvedPathSegment, UnknownPath,
};
pub(crate) use resolved_type::{ResolvedPathType, ResolvedType, Tuple, TypeReference};

mod callable;
mod callable_path;
mod resolved_path;
mod resolved_type;

// E.g. `["std", "path", "PathBuf"]`.
pub type ImportPath = Vec<String>;
