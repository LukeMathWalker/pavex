pub use krate_name::{
    CrateNameResolutionError, UnknownCrate, UnknownDependency, dependency_name2package_id,
    krate2package_id,
};
pub(crate) use resolved_type::{
    Callable, CallableInput, CanonicalType, EnumVariantConstructorPath, EnumVariantInit, FnHeader,
    GenericArgument, GenericLifetimeParameter, InherentMethod, InherentMethodPath, Lifetime,
    LifetimeGenerator, PathType, PathTypeExt, RustIdentifier, StructLiteralInit, TraitMethod,
    TraitMethodPath, Type, TypeReference, get_err_variant, get_ok_variant,
};

mod krate_name;
mod resolved_type;
