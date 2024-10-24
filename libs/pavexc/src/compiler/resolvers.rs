//! Given the fully qualified path to a function (be it a constructor or a handler),
//! find the corresponding item ("resolution") in `rustdoc`'s JSON output to determine
//! its input parameters and output type.
use std::ops::Deref;
use std::sync::Arc;

use ahash::{HashMap, HashMapExt};
use anyhow::anyhow;
use guppy::PackageId;
use once_cell::sync::OnceCell;
use rustdoc_types::{GenericArg, GenericArgs, GenericParamDefKind, ItemEnum, Type};

use crate::language::{
    Callable, Generic, GenericArgument, GenericLifetimeParameter, InvocationStyle, PathType,
    ResolvedPath, ResolvedPathGenericArgument, ResolvedPathLifetime, ResolvedPathSegment,
    ResolvedPathType, ResolvedType, Slice, Tuple, TypeReference, UnknownPath,
};
use crate::rustdoc::{CannotGetCrateData, ResolvedItemWithParent, RustdocKindExt};
use crate::rustdoc::{CrateCollection, ResolvedItem};

use super::utils::process_framework_path;

pub(crate) fn resolve_type(
    type_: &Type,
    // The package id where the type we are trying to process has been referenced (e.g. as an
    // input/output parameter).
    used_by_package_id: &PackageId,
    krate_collection: &CrateCollection,
    generic_bindings: &HashMap<String, ResolvedType>,
) -> Result<ResolvedType, anyhow::Error> {
    match type_ {
        Type::ResolvedPath(rustdoc_types::Path { id, args, name }) => {
            let re_exporter_crate_name = {
                let mut re_exporter = None;
                if let Some(krate) = krate_collection.get_crate_by_package_id(used_by_package_id) {
                    if let Some(item) = krate.maybe_get_type_by_local_type_id(id) {
                        // 0 is the crate index of local types.
                        if item.crate_id == 0 {
                            re_exporter = Some(None);
                        }
                    }
                }
                if re_exporter.is_none() {
                    // It is not guaranteed that this type will be from a direct dependency of `used_by_package_id`.
                    // It might be a re-export from a transitive dependency, done by a direct dependency.
                    // Unfortunately, `rustdoc` does not provide the package id of the crate where the type
                    // was re-exported from, creating a "missing link".
                    // We try to infer it from the `name` property, which is usually the fully qualified
                    // name of the type, e.g. `std::collections::HashMap`.
                    re_exporter = Some(name.split("::").next());
                }
                re_exporter.unwrap()
            };
            let (global_type_id, base_type) = krate_collection
                .get_canonical_path_by_local_type_id(
                    used_by_package_id,
                    id,
                    re_exporter_crate_name,
                )?;
            let type_item = krate_collection.get_type_by_global_type_id(&global_type_id);
            // We want to remove any indirections (e.g. `type Foo = Bar;`) and get the actual type.
            if let ItemEnum::TypeAlias(type_alias) = &type_item.inner {
                let mut alias_generic_bindings = HashMap::new();
                // The generic arguments that have been passed to the type alias.
                // E.g. `u32` in `Foo<u32>` for `type Foo<T=u64> = Bar<T>;`
                let generic_args = if let Some(args) = args {
                    if let GenericArgs::AngleBracketed { args, .. } = args.deref() {
                        Some(args)
                    } else {
                        None
                    }
                } else {
                    None
                };
                // The generic parameters that have been defined for the type alias.
                // E.g. `T` in `type Foo<T> = Bar<T, u64>;`
                let generic_param_defs = &type_alias.generics.params;
                for (i, generic_param_def) in generic_param_defs.iter().enumerate() {
                    // We also try to handle generic parameters, as long as they have a default value.
                    match &generic_param_def.kind {
                        GenericParamDefKind::Type { default, .. } => {
                            let provided_arg = generic_args.and_then(|v| v.get(i));
                            let generic_type = if let Some(provided_arg) = provided_arg {
                                if let GenericArg::Type(provided_arg) = provided_arg {
                                    resolve_type(
                                        provided_arg,
                                        used_by_package_id,
                                        krate_collection,
                                        &generic_bindings,
                                    )?
                                } else {
                                    anyhow::bail!("Expected `{:?}` to be a generic _type_ parameter, but it wasn't!", provided_arg)
                                }
                            } else if let Some(default) = default {
                                let default = resolve_type(
                                    default,
                                    &global_type_id.package_id,
                                    krate_collection,
                                    &generic_bindings,
                                )?;
                                if skip_default(krate_collection, &default) {
                                    continue;
                                }
                                default
                            } else {
                                ResolvedType::Generic(Generic {
                                    name: generic_param_def.name.clone(),
                                })
                            };
                            alias_generic_bindings
                                .insert(generic_param_def.name.to_string(), generic_type);
                        }
                        GenericParamDefKind::Const { .. }
                        | GenericParamDefKind::Lifetime { .. } => {
                            anyhow::bail!("I can only work with generic type parameters when working with type aliases. I can't handle a `{:?}` yet, sorry!", generic_param_def)
                        }
                    }
                }
                let type_ = resolve_type(
                    &type_alias.type_,
                    &global_type_id.package_id,
                    krate_collection,
                    &alias_generic_bindings,
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
                            // when using the type as part of a function signature—e.g.
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
                            .as_slice();
                            for (i, arg_def) in generic_arg_defs.iter().enumerate() {
                                let generic_argument = match &arg_def.kind {
                                    GenericParamDefKind::Lifetime { .. } => {
                                        let mut lifetime_name = arg_def.name.clone();
                                        if let Some(arg) = args.get(i) {
                                            if let GenericArg::Lifetime(l) = &arg {
                                                lifetime_name = l.clone();
                                            }
                                        }
                                        if lifetime_name == "'static" {
                                            GenericArgument::Lifetime(
                                                GenericLifetimeParameter::Static,
                                            )
                                        } else {
                                            let name = lifetime_name.trim_start_matches('\'');
                                            let lifetime = if name == "_" {
                                                GenericLifetimeParameter::Named("_".into())
                                            } else {
                                                GenericLifetimeParameter::Named(name.to_owned())
                                            };
                                            GenericArgument::Lifetime(lifetime)
                                        }
                                    }
                                    GenericParamDefKind::Type { default, .. } => {
                                        if let Some(GenericArg::Type(generic_type)) = args.get(i) {
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
                                        } else if let Some(default) = default {
                                            let default = resolve_type(
                                                &default,
                                                &global_type_id.package_id,
                                                krate_collection,
                                                &generic_bindings,
                                            )?;
                                            if skip_default(krate_collection, &default) {
                                                continue;
                                            }
                                            GenericArgument::TypeParameter(default)
                                        } else {
                                            GenericArgument::TypeParameter(ResolvedType::Generic(
                                                Generic {
                                                    name: arg_def.name.clone(),
                                                },
                                            ))
                                        }
                                    }
                                    GenericParamDefKind::Const { .. } => {
                                        return Err(anyhow!(
                                            "I don't support const generics in types yet. Sorry!"
                                        ));
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
                let t = PathType {
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
            type_,
            is_mutable,
        } => {
            let resolved_type = resolve_type(
                type_,
                used_by_package_id,
                krate_collection,
                generic_bindings,
            )?;
            let t = TypeReference {
                is_mutable: *is_mutable,
                lifetime: lifetime.to_owned().into(),
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
    let (
        ResolvedItemWithParent {
            item: callable,
            parent,
        },
        qualified_self_type,
    ) = callable_path.find_rustdoc_items(krate_collection)?;
    let used_by_package_id = &callable_path.package_id;
    let (header, decl, fn_generics_defs, invocation_style) = match &callable.item.inner {
        ItemEnum::Function(f) => (
            &f.header,
            &f.sig,
            &f.generics,
            InvocationStyle::FunctionCall,
        ),
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
    let parent_info = parent.map(|p| {
        let parent_segments = callable_path.segments[..callable_path.segments.len() - 1].to_vec();
        let parent_path = ResolvedPath {
            segments: parent_segments,
            qualified_self: callable_path.qualified_self.clone(),
            package_id: callable_path.package_id.clone(),
        };
        (p, parent_path)
    });
    if let Some((parent, parent_path)) = &parent_info {
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
            match resolve_type_path_with_item(parent_path, parent, krate_collection) {
                Ok(parent_type) => {
                    generic_bindings.insert("Self".to_string(), parent_type);
                }
                Err(e) => {
                    tracing::trace!(error.msg = %e, error.details = ?e, "Error resolving the parent type");
                }
            }
        }
    }
    let fn_generic_args = &callable_path.segments.last().unwrap().generic_arguments;
    for (generic_arg, generic_def) in fn_generic_args.iter().zip(&fn_generics_defs.params) {
        let generic_name = &generic_def.name;
        let generic_type = match generic_arg {
            ResolvedPathGenericArgument::Type(t) => t,
            _ => {
                continue;
            }
        };
        let resolved_type = generic_type.resolve(krate_collection).map_err(|e| {
            GenericParameterResolutionError {
                generic_type: generic_type.to_owned(),
                callable_path: callable_path.to_owned(),
                callable_item: callable.item.clone().into_owned(),
                source: Arc::new(e),
            }
        })?;
        generic_bindings.insert(generic_name.to_owned(), resolved_type);
    }

    let mut resolved_parameter_types = Vec::with_capacity(decl.inputs.len());
    let mut takes_self_as_ref = false;
    for (parameter_index, (_, parameter_type)) in decl.inputs.iter().enumerate() {
        if parameter_index == 0 {
            // The first parameter might be `&self` or `&mut self`.
            // This is important to know for carrying out further analysis doing the line,
            // e.g. undoing lifetime elision.
            if let Type::BorrowedRef { type_, .. } = parameter_type {
                if let Type::Generic(g) = type_.deref() {
                    if g == "Self" {
                        takes_self_as_ref = true;
                    }
                }
            }
        }
        match resolve_type(
            parameter_type,
            used_by_package_id,
            krate_collection,
            &generic_bindings,
        ) {
            Ok(p) => resolved_parameter_types.push(p),
            Err(e) => {
                return Err(InputParameterResolutionError {
                    parameter_type: parameter_type.to_owned(),
                    callable_path: callable_path.to_owned(),
                    callable_item: callable.item.into_owned(),
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
                        callable_item: callable.item.into_owned(),
                        source: Arc::new(e),
                    }
                    .into());
                }
            }
        }
    };

    // We need to make sure that we always refer to user-registered types using a public path, even though they
    // might have been registered using a private path (e.g. `self::...` where `self` is a private module).
    // For this reason, we fall back to the canonical path—i.e. the shortest public path for the given item.
    // TODO: We should do the same for qualified self, if it's populated.
    let canonical_path = {
        // If the item is a method, we start by finding the canonical path to its parent—i.e. the struct/enum/trait
        // to which the method is attached.
        let parent_canonical_path = match parent_info {
            Some((parent, parent_path)) => {
                match krate_collection.get_canonical_path_by_global_type_id(&parent.item_id) {
                    Ok(canonical_segments) => {
                        let mut segments: Vec<_> = canonical_segments
                            .into_iter()
                            .map(|s| ResolvedPathSegment {
                                ident: s.into(),
                                generic_arguments: vec![],
                            })
                            .collect();
                        // The canonical path doesn't include the (populated or omitted) generic arguments from the user-provided path,
                        // so we need to add them back in.
                        segments.last_mut().unwrap().generic_arguments = parent_path
                            .segments
                            .last()
                            .unwrap()
                            .generic_arguments
                            .clone();
                        Some(ResolvedPath {
                            segments,
                            qualified_self: parent_path.qualified_self.clone(),
                            package_id: parent.item_id.package_id.clone(),
                        })
                    }
                    Err(_) => {
                        tracing::warn!(
                            package_id = parent.item_id.package_id.repr(),
                            item_id = ?parent.item_id.rustdoc_item_id,
                            "Failed to find canonical path for {:?}",
                            parent_path
                        );
                        Some(parent_path)
                    }
                }
            }
            _ => None,
        };

        let canonical_path = match parent_canonical_path {
            Some(p) => {
                // We have already canonicalized the parent path, so we just need to append the method name and we're done.
                let mut segments = p.segments;
                segments.push(callable_path.segments.last().unwrap().clone());
                ResolvedPath {
                    segments,
                    qualified_self: callable_path.qualified_self.clone(),
                    package_id: p.package_id.clone(),
                }
            }
            None => {
                // There was no parent, it's a free function or a straight struct/enum. We need to go through the same process
                // we applied for the parent.
                match krate_collection.get_canonical_path_by_global_type_id(&callable.item_id) {
                    Ok(p) => {
                        let mut segments: Vec<_> = p
                            .into_iter()
                            .map(|s| ResolvedPathSegment {
                                ident: s.into(),
                                generic_arguments: vec![],
                            })
                            .collect();
                        // The canonical path doesn't include the (populated or omitted) generic arguments from the user-provided callable path,
                        // so we need to add them back in.
                        segments.last_mut().unwrap().generic_arguments = callable_path
                            .segments
                            .last()
                            .unwrap()
                            .generic_arguments
                            .clone();
                        ResolvedPath {
                            segments,
                            qualified_self: callable_path.qualified_self.clone(),
                            package_id: callable.item_id.package_id.clone(),
                        }
                    }
                    Err(_) => {
                        tracing::warn!(
                            package_id = callable.item_id.package_id.repr(),
                            item_id = ?callable.item_id.rustdoc_item_id,
                            "Failed to find canonical path for {:?}",
                            callable_path
                        );
                        callable_path.to_owned()
                    }
                }
            }
        };
        canonical_path
    };

    let callable = Callable {
        is_async: header.is_async,
        takes_self_as_ref,
        output: output_type_path,
        path: canonical_path,
        inputs: resolved_parameter_types,
        invocation_style,
        source_coordinates: Some(callable.item_id),
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
    let ItemEnum::Trait(trait_item) = inner else {
        unreachable!()
    };
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
    krate_collection: &CrateCollection,
) -> Result<ResolvedType, TypeResolutionError> {
    fn _resolve_type_path(
        path: &ResolvedPath,
        krate_collection: &CrateCollection,
    ) -> Result<ResolvedType, anyhow::Error> {
        let (item, _) = path.find_rustdoc_items(krate_collection)?;
        resolve_type_path_with_item(&path, &item.item, krate_collection)
    }

    _resolve_type_path(path, krate_collection).map_err(|source| TypeResolutionError {
        path: path.clone(),
        source,
    })
}

pub(crate) fn resolve_type_path_with_item(
    path: &ResolvedPath,
    resolved_item: &ResolvedItem,
    krate_collection: &CrateCollection,
) -> Result<ResolvedType, anyhow::Error> {
    let item = &resolved_item.item;
    let used_by_package_id = resolved_item.item_id.package_id();
    let (global_type_id, base_type) =
        krate_collection.get_canonical_path_by_local_type_id(used_by_package_id, &item.id, None)?;
    let mut generic_arguments = vec![];
    let (last_segment, first_segments) = path.segments.split_last().unwrap();
    for segment in first_segments {
        for generic_path in &segment.generic_arguments {
            let arg = match generic_path {
                ResolvedPathGenericArgument::Type(t) => {
                    GenericArgument::TypeParameter(t.resolve(krate_collection)?)
                }
                ResolvedPathGenericArgument::Lifetime(l) => match l {
                    ResolvedPathLifetime::Static => {
                        GenericArgument::Lifetime(GenericLifetimeParameter::Static)
                    }
                    ResolvedPathLifetime::Named(name) => {
                        GenericArgument::Lifetime(GenericLifetimeParameter::Named(name.clone()))
                    }
                },
            };
            generic_arguments.push(arg);
        }
    }
    // Some generic parameters might not be explicitly specified in the path, so we need to
    // look at the definition of the type to take them into account.
    let generic_defs = match &resolved_item.item.inner {
        ItemEnum::Struct(s) => &s.generics.params,
        ItemEnum::Enum(e) => &e.generics.params,
        ItemEnum::Trait(t) => &t.generics.params,
        _ => unreachable!(),
    };
    for (i, generic_def) in generic_defs.iter().enumerate() {
        let arg = if let Some(generic_path) = last_segment.generic_arguments.get(i) {
            match generic_path {
                ResolvedPathGenericArgument::Type(t) => {
                    GenericArgument::TypeParameter(t.resolve(krate_collection)?)
                }
                ResolvedPathGenericArgument::Lifetime(l) => match l {
                    ResolvedPathLifetime::Static => {
                        GenericArgument::Lifetime(GenericLifetimeParameter::Static)
                    }
                    ResolvedPathLifetime::Named(name) => {
                        GenericArgument::Lifetime(GenericLifetimeParameter::Named(name.clone()))
                    }
                },
            }
        } else {
            match &generic_def.kind {
                GenericParamDefKind::Lifetime { .. } => {
                    let lifetime_name = generic_def.name.trim_start_matches('\'');
                    if lifetime_name == "static" {
                        GenericArgument::Lifetime(GenericLifetimeParameter::Static)
                    } else {
                        GenericArgument::Lifetime(GenericLifetimeParameter::Named(
                            lifetime_name.to_owned(),
                        ))
                    }
                }
                GenericParamDefKind::Type { default, .. } => {
                    if let Some(default) = default {
                        let default = resolve_type(
                            default,
                            used_by_package_id,
                            krate_collection,
                            &HashMap::new(),
                        )?;
                        if skip_default(krate_collection, &default) {
                            continue;
                        }
                        GenericArgument::TypeParameter(default)
                    } else {
                        GenericArgument::TypeParameter(ResolvedType::Generic(Generic {
                            name: generic_def.name.clone(),
                        }))
                    }
                }
                GenericParamDefKind::Const { .. } => {
                    unimplemented!("const generic parameters are not supported yet")
                }
            }
        };
        generic_arguments.push(arg);
    }
    Ok(PathType {
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
    GenericParameterResolutionError(#[from] GenericParameterResolutionError),
    #[error(transparent)]
    InputParameterResolutionError(#[from] InputParameterResolutionError),
    #[error(transparent)]
    OutputTypeResolutionError(#[from] OutputTypeResolutionError),
    #[error(transparent)]
    CannotGetCrateData(#[from] CannotGetCrateData),
}

#[derive(Debug, thiserror::Error)]
#[error("I can't resolve `{path}` to a type.")]
pub(crate) struct TypeResolutionError {
    path: ResolvedPath,
    #[source]
    pub source: anyhow::Error,
}

#[derive(Debug, thiserror::Error, Clone)]
#[error("I can work with functions and methods, but `{import_path}` is neither.\nIt is {item_kind} and I don't know how to handle it here.")]
pub(crate) struct UnsupportedCallableKind {
    pub import_path: ResolvedPath,
    pub item_kind: String,
}

#[derive(Debug, thiserror::Error, Clone)]
#[error("One of the input parameters for `{callable_path}` has a type that I can't handle.")]
pub(crate) struct InputParameterResolutionError {
    pub callable_path: ResolvedPath,
    pub callable_item: rustdoc_types::Item,
    pub parameter_type: Type,
    pub parameter_index: usize,
    #[source]
    pub source: Arc<anyhow::Error>,
}

#[derive(Debug, thiserror::Error, Clone)]
#[error("I can't handle `{generic_type}`, one of the generic parameters you specified for `{callable_path}`.")]
pub(crate) struct GenericParameterResolutionError {
    pub callable_path: ResolvedPath,
    pub callable_item: rustdoc_types::Item,
    pub generic_type: ResolvedPathType,
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

/// This is a gigantic hack to work around an issue with `std`'s collections:
/// they are all generic over the allocator type, but the default (`alloc::alloc::Global`)
/// is a nightly-only type.
/// If you spell it out, the code won't compile on stable, even though it does
/// exactly the same thing as omitting the parameter.
fn skip_default(krate_collection: &CrateCollection, default: &ResolvedType) -> bool {
    static GLOBAL_ALLOCATOR: OnceCell<ResolvedType> = OnceCell::new();

    let alloc = GLOBAL_ALLOCATOR.get_or_init(|| {
        process_framework_path(
            "alloc::alloc::Global",
            krate_collection.package_graph(),
            krate_collection,
        )
    });
    alloc == default
}
