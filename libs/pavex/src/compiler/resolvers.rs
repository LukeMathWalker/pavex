//! Given the fully qualified path to a function (be it a constructor or a handler),
//! find the corresponding item ("resolution") in `rustdoc`'s JSON output to determine
//! its input parameters and output type.
use std::sync::Arc;

use ahash::{HashMap, HashMapExt};
use anyhow::anyhow;
use guppy::PackageId;
use rustdoc_types::{GenericArg, GenericArgs, GenericParamDefKind, ItemEnum, Type};

use crate::language::{
    Callable, Generic, GenericArgument, InvocationStyle, Lifetime, ResolvedPath,
    ResolvedPathGenericArgument, ResolvedPathLifetime, ResolvedPathType, ResolvedType, Slice,
    Tuple, TypeReference, UnknownPath,
};
use crate::rustdoc::{CannotGetCrateData, RustdocKindExt};
use crate::rustdoc::{CrateCollection, ResolvedItem};

pub(crate) fn resolve_type(
    type_: &Type,
    // The package id where the type we are trying to process has been referenced (e.g. as an
    // input/output parameter).
    used_by_package_id: &PackageId,
    krate_collection: &CrateCollection,
    generic_bindings: &HashMap<String, ResolvedType>,
) -> Result<ResolvedType, anyhow::Error> {
    match type_ {
        Type::ResolvedPath(rustdoc_types::Path { id, args, .. }) => {
            let (global_type_id, base_type) =
                krate_collection.get_canonical_path_by_local_type_id(used_by_package_id, id)?;
            let type_item = krate_collection.get_type_by_global_type_id(&global_type_id);
            // We want to remove any indirections (e.g. `type Foo = Bar;`) and get the actual type.
            if let ItemEnum::Typedef(typedef) = &type_item.inner {
                let mut generic_bindings = HashMap::new();
                for generic in &typedef.generics.params {
                    // We also try to handle generic parameters, as long as they have a default value.
                    match &generic.kind {
                        GenericParamDefKind::Type {
                            default: Some(default),
                            ..
                        } => {
                            let default = resolve_type(
                                default,
                                &global_type_id.package_id,
                                krate_collection,
                                &generic_bindings,
                            )?;
                            generic_bindings.insert(generic.name.to_string(), default);
                        }
                        GenericParamDefKind::Type { default: None, .. }
                        | GenericParamDefKind::Const { .. }
                        | GenericParamDefKind::Lifetime { .. } => {
                            anyhow::bail!("I can't only generic type parameters with a default when working with type aliases. I can't handle a `{:?}` yet, sorry!", generic)
                        }
                    }
                }
                let type_ = resolve_type(
                    &typedef.type_,
                    &global_type_id.package_id,
                    krate_collection,
                    &generic_bindings,
                )?;
                Ok(type_)
            } else {
                let mut generics = vec![];
                if let Some(args) = args {
                    match &**args {
                        GenericArgs::AngleBracketed { args, .. } => {
                            // We fetch the name of the generic parameters as they appear
                            // in the definition of the type that we are processing.
                            // This is necessary because generic parameters can be elided
                            // when using the type as part of a function signature - e.g.
                            // `fn path(params: Params<'_, '_>) -> Result<_, _> { ... }`
                            //
                            // Can the two elided generic lifetime parameters be set to two
                            // different values? Or must they be the same?
                            // We need to check the definition of `Params` to find out.
                            let generic_arg_defs = match &type_item.inner {
                                ItemEnum::Struct(s) => &s.generics,
                                ItemEnum::Enum(e) => &e.generics,
                                _ => unreachable!(),
                            }
                            .params
                            .iter()
                            .map(|p| p.name.trim_start_matches('\'').to_string())
                            .collect::<Vec<_>>();
                            for (arg, arg_def_name) in args.iter().zip(generic_arg_defs) {
                                let generic_argument = match arg {
                                    GenericArg::Lifetime(l) => {
                                        if l == "'static" {
                                            GenericArgument::Lifetime(Lifetime::Static)
                                        } else {
                                            let name = l.trim_start_matches('\'');
                                            let lifetime = if name == "_" {
                                                // TODO: we must make sure to choose a unique
                                                //  name for this lifetime.
                                                //  As in, one that is not used by any other lifetime
                                                //  in the context of the function we are processing.
                                                Lifetime::Named(format!("_{arg_def_name}"))
                                            } else {
                                                Lifetime::Named(name.to_owned())
                                            };
                                            GenericArgument::Lifetime(lifetime)
                                        }
                                    }
                                    GenericArg::Type(generic_type) => {
                                        if let Type::Generic(generic) = generic_type {
                                            if let Some(resolved_type) =
                                                generic_bindings.get(generic)
                                            {
                                                GenericArgument::TypeParameter(
                                                    resolved_type.to_owned(),
                                                )
                                            } else {
                                                GenericArgument::TypeParameter(
                                                    ResolvedType::Generic(Generic {
                                                        name: generic.to_owned(),
                                                    }),
                                                )
                                            }
                                        } else {
                                            GenericArgument::TypeParameter(resolve_type(
                                                generic_type,
                                                used_by_package_id,
                                                krate_collection,
                                                generic_bindings,
                                            )?)
                                        }
                                    }
                                    GenericArg::Const(_) => {
                                        return Err(anyhow!(
                                            "I don't support const generics in types yet. Sorry!"
                                        ));
                                    }
                                    GenericArg::Infer => {
                                        return Err(anyhow!("I don't support inferred generic arguments in types yet. Sorry!"));
                                    }
                                };
                                generics.push(generic_argument);
                            }
                        }
                        GenericArgs::Parenthesized { .. } => {
                            return Err(anyhow!("I don't support function pointers yet. Sorry!"));
                        }
                    }
                }
                let t = ResolvedPathType {
                    package_id: global_type_id.package_id().to_owned(),
                    rustdoc_id: Some(global_type_id.rustdoc_item_id),
                    base_type: base_type.to_vec(),
                    generic_arguments: generics,
                };
                Ok(ResolvedType::ResolvedPath(t))
            }
        }
        Type::BorrowedRef {
            lifetime,
            mutable,
            type_,
        } => {
            if *mutable {
                return Err(anyhow!(
                    "Mutable references are not allowed. You can only pass an argument \
                    by value (`move` semantic) or via a shared reference (`&MyType`)",
                ));
            }
            let resolved_type = resolve_type(
                type_,
                used_by_package_id,
                krate_collection,
                generic_bindings,
            )?;
            let t = TypeReference {
                is_mutable: *mutable,
                is_static: lifetime.as_ref().map(|l| l == "'static").unwrap_or(false),
                inner: Box::new(resolved_type),
            };
            Ok(t.into())
        }
        Type::Generic(s) => {
            if let Some(resolved_type) = generic_bindings.get(s) {
                Ok(resolved_type.to_owned())
            } else {
                Ok(ResolvedType::Generic(Generic { name: s.to_owned() }))
            }
        }
        Type::Tuple(t) => {
            let mut types = Vec::with_capacity(t.len());
            for type_ in t {
                types.push(resolve_type(
                    type_,
                    used_by_package_id,
                    krate_collection,
                    generic_bindings,
                )?);
            }
            Ok(ResolvedType::Tuple(Tuple { elements: types }))
        }
        Type::Primitive(p) => Ok(ResolvedType::ScalarPrimitive(p.as_str().try_into()?)),
        Type::Slice(type_) => {
            let inner = resolve_type(
                type_,
                used_by_package_id,
                krate_collection,
                generic_bindings,
            )?;
            Ok(ResolvedType::Slice(Slice {
                element_type: Box::new(inner),
            }))
        }
        _ => Err(anyhow!(
            "I can't handle this kind ({:?}) of type yet. Sorry!",
            type_
        )),
    }
}

pub(crate) fn resolve_callable(
    krate_collection: &CrateCollection,
    callable_path: &ResolvedPath,
) -> Result<Callable, CallableResolutionError> {
    let (callable_type, qualified_self_type) =
        callable_path.find_rustdoc_items(krate_collection)?;
    let used_by_package_id = &callable_path.package_id;
    let (header, decl, invocation_style) = match &callable_type.item.item.inner {
        ItemEnum::Function(f) => (&f.header, &f.decl, InvocationStyle::FunctionCall),
        kind => {
            let item_kind = kind.kind().to_owned();
            return Err(UnsupportedCallableKind {
                import_path: callable_path.to_owned(),
                item_kind,
            }
            .into());
        }
    };

    let mut generic_bindings = HashMap::new();
    if let Some(qself) = qualified_self_type {
        generic_bindings.insert("Self".to_string(), qself);
    }
    if let Some(parent) = &callable_type.parent {
        let parent_segments = callable_path.segments[..callable_path.segments.len() - 1].to_vec();
        let parent_path = ResolvedPath {
            segments: parent_segments,
            qualified_self: callable_path.qualified_self.clone(),
            package_id: callable_path.package_id.clone(),
        };
        if matches!(parent.item.inner, ItemEnum::Trait(_)) {
            if let Err(e) = get_trait_generic_bindings(
                parent,
                &parent_path,
                krate_collection,
                &mut generic_bindings,
            ) {
                tracing::trace!(error.msg = %e, error.details = ?e, "Error getting trait generic bindings");
            }
        } else {
            match resolve_type_path(&parent_path, parent, krate_collection) {
                Ok(parent_type) => {
                    generic_bindings.insert("Self".to_string(), parent_type);
                }
                Err(e) => {
                    tracing::trace!(error.msg = %e, error.details = ?e, "Error resolving the parent type");
                }
            }
        }
    }

    let mut parameter_paths = Vec::with_capacity(decl.inputs.len());
    for (parameter_index, (_, parameter_type)) in decl.inputs.iter().enumerate() {
        match resolve_type(
            parameter_type,
            used_by_package_id,
            krate_collection,
            &generic_bindings,
        ) {
            Ok(p) => parameter_paths.push(p),
            Err(e) => {
                return Err(ParameterResolutionError {
                    parameter_type: parameter_type.to_owned(),
                    callable_path: callable_path.to_owned(),
                    callable_item: callable_type.item.item.into_owned(),
                    source: Arc::new(e),
                    parameter_index,
                }
                .into());
            }
        }
    }
    let output_type_path = match &decl.output {
        // Unit type
        None => None,
        Some(output_type) => {
            match resolve_type(
                output_type,
                used_by_package_id,
                krate_collection,
                &generic_bindings,
            ) {
                Ok(p) => Some(p),
                Err(e) => {
                    return Err(OutputTypeResolutionError {
                        output_type: output_type.to_owned(),
                        callable_path: callable_path.to_owned(),
                        callable_item: callable_type.item.item.into_owned(),
                        source: Arc::new(e),
                    }
                    .into());
                }
            }
        }
    };
    let callable = Callable {
        is_async: header.async_,
        output: output_type_path,
        path: callable_path.to_owned(),
        inputs: parameter_paths,
        invocation_style,
        source_coordinates: Some(callable_type.item.item_id),
    };
    Ok(callable)
}

fn get_trait_generic_bindings(
    resolved_item: &ResolvedItem,
    path: &ResolvedPath,
    krate_collection: &CrateCollection,
    generic_bindings: &mut HashMap<String, ResolvedType>,
) -> Result<(), anyhow::Error> {
    let inner = &resolved_item.item.inner;
    let ItemEnum::Trait(trait_item) = inner else { unreachable!() };
    // TODO: handle defaults
    for (generic_slot, assigned_parameter) in trait_item
        .generics
        .params
        .iter()
        .zip(path.segments.last().unwrap().generic_arguments.iter())
    {
        if let ResolvedPathGenericArgument::Type(t) = assigned_parameter {
            // TODO: handle conflicts
            generic_bindings.insert(generic_slot.name.clone(), t.resolve(krate_collection)?);
        }
    }
    Ok(())
}

pub(crate) fn resolve_type_path(
    path: &ResolvedPath,
    resolved_item: &ResolvedItem,
    krate_collection: &CrateCollection,
) -> Result<ResolvedType, anyhow::Error> {
    let item = &resolved_item.item;
    let used_by_package_id = resolved_item.item_id.package_id();
    let (global_type_id, base_type) =
        krate_collection.get_canonical_path_by_local_type_id(used_by_package_id, &item.id)?;
    let mut generic_arguments = vec![];
    for segment in &path.segments {
        for generic_path in &segment.generic_arguments {
            let arg = match generic_path {
                ResolvedPathGenericArgument::Type(t) => {
                    GenericArgument::TypeParameter(t.resolve(krate_collection)?)
                }
                ResolvedPathGenericArgument::Lifetime(l) => match l {
                    ResolvedPathLifetime::Static => GenericArgument::Lifetime(Lifetime::Static),
                    ResolvedPathLifetime::Named(name) => {
                        GenericArgument::Lifetime(Lifetime::Named(name.clone()))
                    }
                },
            };
            generic_arguments.push(arg);
        }
    }
    Ok(ResolvedPathType {
        package_id: global_type_id.package_id().to_owned(),
        rustdoc_id: Some(global_type_id.rustdoc_item_id),
        base_type: base_type.to_vec(),
        generic_arguments,
    }
    .into())
}

#[derive(thiserror::Error, Debug, Clone)]
pub(crate) enum CallableResolutionError {
    #[error(transparent)]
    UnsupportedCallableKind(#[from] UnsupportedCallableKind),
    #[error(transparent)]
    UnknownCallable(#[from] UnknownPath),
    #[error(transparent)]
    ParameterResolutionError(#[from] ParameterResolutionError),
    #[error(transparent)]
    OutputTypeResolutionError(#[from] OutputTypeResolutionError),
    #[error(transparent)]
    CannotGetCrateData(#[from] CannotGetCrateData),
}

#[derive(Debug, thiserror::Error, Clone)]
#[error("I can work with functions and methods, but `{import_path}` is neither.\nIt is {item_kind} and I don't know how to handle it here.")]
pub(crate) struct UnsupportedCallableKind {
    pub import_path: ResolvedPath,
    pub item_kind: String,
}

#[derive(Debug, thiserror::Error, Clone)]
#[error("One of the input parameters for `{callable_path}` has a type that I can't handle.")]
pub(crate) struct ParameterResolutionError {
    pub callable_path: ResolvedPath,
    pub callable_item: rustdoc_types::Item,
    pub parameter_type: Type,
    pub parameter_index: usize,
    #[source]
    pub source: Arc<anyhow::Error>,
}

#[derive(Debug, thiserror::Error, Clone)]
#[error("I don't know how to handle the type returned by `{callable_path}`.")]
pub(crate) struct OutputTypeResolutionError {
    pub callable_path: ResolvedPath,
    pub callable_item: rustdoc_types::Item,
    pub output_type: Type,
    #[source]
    pub source: Arc<anyhow::Error>,
}
