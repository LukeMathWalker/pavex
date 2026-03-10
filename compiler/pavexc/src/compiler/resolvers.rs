//! Given the fully qualified path to a function (be it a constructor or a handler),
//! find the corresponding item ("resolution") in `rustdoc`'s JSON output to determine
//! its input parameters and output type.
use std::ops::Deref;
use std::sync::Arc;

use rustdoc_types::{GenericParamDefKind, ItemEnum, Type as RustdocType};
use tracing_log_error::log_error;

use crate::language::{
    Callable, CallableInput, CallableItem, FQGenericArgument, FQPath, FQPathSegment, FQPathType,
    FnHeader, FreeFunction, FreeFunctionPath, Generic, GenericArgument, GenericLifetimeParameter,
    InherentMethod, InherentMethodPath, PathType, RustIdentifier, TraitMethod, TraitMethodPath,
    Type, UnknownPath, find_rustdoc_callable_items, find_rustdoc_item_type, resolve_fq_path_type,
};
use crate::rustdoc::{CannotGetCrateData, CrateCollection, ResolvedItem};
use rustdoc_ext::RustdocKindExt;

// Re-export types that moved to `rustdoc_resolver` so that downstream code
// within pavexc can keep importing them from this module.
pub use rustdoc_resolver::{
    GenericBindings, InputParameterResolutionError, OutputTypeResolutionError, SelfResolutionError,
    TypeResolutionError, UnsupportedConstGeneric, resolve_type,
};

pub(crate) fn resolve_callable(
    krate_collection: &CrateCollection,
    callable_path: &FQPath,
) -> Result<Callable, CallableResolutionError> {
    let callable_items = find_rustdoc_callable_items(callable_path, krate_collection)??;
    let (callable_item, new_callable_path) = match &callable_items {
        CallableItem::Function(item, p) => (item, p),
        CallableItem::Method { method, .. } => (&method.0, &method.1),
    };
    let used_by_package_id = &new_callable_path.package_id;

    let (header, decl, fn_generics_defs) = match &callable_item.item.inner {
        ItemEnum::Function(f) => (&f.header, &f.sig, &f.generics),
        kind => {
            let item_kind = kind.kind().to_owned();
            return Err(UnsupportedCallableKind {
                import_path: callable_path.to_owned(),
                item_kind,
            }
            .into());
        }
    };

    let mut generic_bindings = GenericBindings::default();
    if let CallableItem::Method {
        method_owner,
        qualified_self,
        ..
    } = &callable_items
    {
        if matches!(&method_owner.0.item.inner, ItemEnum::Trait(_))
            && let Err(e) = get_trait_generic_bindings(
                &method_owner.0,
                &method_owner.1,
                krate_collection,
                &mut generic_bindings,
            )
        {
            log_error!(*e, level: tracing::Level::WARN, "Error getting trait generic bindings");
        }

        let self_ = match qualified_self {
            Some(q) => q,
            None => method_owner,
        };

        let self_generic_ty =
            match resolve_type_path_with_item(&self_.1, &self_.0, krate_collection) {
                Ok(ty) => Some(ty),
                Err(e) => {
                    log_error!(*e, level: tracing::Level::WARN, "Error resolving the `Self` type");
                    None
                }
            };
        if let Some(ty) = self_generic_ty {
            generic_bindings.types.insert("Self".to_string(), ty);
        }
    }

    let fn_generic_args = &new_callable_path.segments.last().unwrap().generic_arguments;
    for (generic_arg, generic_def) in fn_generic_args.iter().zip(&fn_generics_defs.params) {
        let generic_name = &generic_def.name;
        match generic_arg {
            FQGenericArgument::Type(t) => {
                let resolved_type = resolve_fq_path_type(t, krate_collection).map_err(|e| {
                    GenericParameterResolutionError {
                        generic_type: t.to_owned(),
                        callable_path: new_callable_path.to_owned(),
                        callable_item: callable_item.item.clone().into_owned(),
                        source: Arc::new(e),
                    }
                })?;
                generic_bindings
                    .types
                    .insert(generic_name.to_owned(), resolved_type);
            }
            FQGenericArgument::Lifetime(l) => {
                let resolved_lifetime = l.to_string();
                generic_bindings
                    .lifetimes
                    .insert(generic_name.to_owned(), resolved_lifetime);
            }
        }
    }

    let mut resolved_parameter_types = Vec::with_capacity(decl.inputs.len());
    let mut takes_self_as_ref = false;
    for (parameter_index, (parameter_name, parameter_type)) in decl.inputs.iter().enumerate() {
        if parameter_index == 0 {
            // The first parameter might be `&self` or `&mut self`.
            // This is important to know for carrying out further analysis doing the line,
            // e.g. undoing lifetime elision.
            if let RustdocType::BorrowedRef { type_, .. } = parameter_type
                && let RustdocType::Generic(g) = type_.deref()
                && g == "Self"
            {
                takes_self_as_ref = true;
            }
        }
        match resolve_type(
            parameter_type,
            used_by_package_id,
            krate_collection,
            &generic_bindings,
        ) {
            Ok(p) => resolved_parameter_types.push(CallableInput {
                name: RustIdentifier::new(parameter_name.clone()),
                type_: p,
            }),
            Err(e) => {
                return Err(InputParameterResolutionError {
                    parameter_type: parameter_type.to_owned(),
                    callable_path: new_callable_path.to_string(),
                    callable_item: callable_item.item.clone().into_owned(),
                    source: Arc::new(e.into()),
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
                        callable_path: new_callable_path.to_string(),
                        callable_item: callable_item.item.clone().into_owned(),
                        source: Arc::new(e.into()),
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
        let parent_canonical_path = match &callable_items {
            CallableItem::Method {
                method_owner: self_,
                ..
            } => {
                match krate_collection.get_canonical_path_by_global_type_id(&self_.0.item_id) {
                    Ok(canonical_segments) => {
                        let mut segments: Vec<_> = canonical_segments
                            .iter()
                            .map(|s| FQPathSegment {
                                ident: s.into(),
                                generic_arguments: vec![],
                            })
                            .collect();
                        // The canonical path doesn't include the (populated or omitted) generic arguments from the user-provided path,
                        // so we need to add them back in.
                        segments.last_mut().unwrap().generic_arguments =
                            self_.1.segments.last().unwrap().generic_arguments.clone();
                        Some(FQPath {
                            segments,
                            qualified_self: self_.1.qualified_self.clone(),
                            package_id: self_.0.item_id.package_id.clone(),
                        })
                    }
                    Err(_) => {
                        tracing::warn!(
                            package_id = self_.0.item_id.package_id.repr(),
                            item_id = ?self_.0.item_id.rustdoc_item_id,
                            "Failed to find canonical path for {:?}",
                            self_.1
                        );
                        Some(self_.1.clone())
                    }
                }
            }
            _ => None,
        };

        match parent_canonical_path {
            Some(p) => {
                // We have already canonicalized the parent path, so we just need to append the method name and we're done.
                let mut segments = p.segments;
                segments.push(callable_path.segments.last().unwrap().clone());
                FQPath {
                    segments,
                    qualified_self: callable_path.qualified_self.clone(),
                    package_id: p.package_id.clone(),
                }
            }
            None => {
                // There was no parent, it's a free function or a straight struct/enum. We need to go through the same process
                // we applied for the parent.
                match krate_collection.get_canonical_path_by_global_type_id(&callable_item.item_id)
                {
                    Ok(p) => {
                        let mut segments: Vec<_> = p
                            .iter()
                            .map(|s| FQPathSegment {
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
                        FQPath {
                            segments,
                            qualified_self: callable_path.qualified_self.clone(),
                            package_id: callable_item.item_id.package_id.clone(),
                        }
                    }
                    Err(_) => {
                        tracing::warn!(
                            package_id = callable_item.item_id.package_id.repr(),
                            item_id = ?callable_item.item_id.rustdoc_item_id,
                            "Failed to find canonical path for {:?}",
                            callable_path
                        );
                        callable_path.to_owned()
                    }
                }
            }
        }
    };

    let symbol_name = callable_item.item.attrs.iter().find_map(|attr| match attr {
        rustdoc_types::Attribute::NoMangle => callable_item.item.name.clone(),
        rustdoc_types::Attribute::ExportName(name) => Some(name.clone()),
        _ => None,
    });

    let resolved_path =
        fq_path_to_resolved_path(&canonical_path, &callable_items, krate_collection)
            .expect("Failed to convert generic arguments when building callable path");

    let fn_header = FnHeader {
        output: output_type_path,
        inputs: resolved_parameter_types,
        is_async: header.is_async,
        abi: header.abi.clone(),
        is_unsafe: header.is_unsafe,
        is_c_variadic: decl.is_c_variadic,
        symbol_name,
    };
    let source_coordinates = Some(callable_item.item_id.clone());

    let callable = match resolved_path {
        ResolvedPath::FreeFunction(path) => Callable::FreeFunction(FreeFunction {
            path,
            header: fn_header,
            source_coordinates,
        }),
        ResolvedPath::InherentMethod(path) => Callable::InherentMethod(InherentMethod {
            path,
            header: fn_header,
            source_coordinates,
            takes_self_as_ref,
        }),
        ResolvedPath::TraitMethod(path) => Callable::TraitMethod(TraitMethod {
            path,
            header: fn_header,
            source_coordinates,
            takes_self_as_ref,
        }),
    };
    Ok(callable)
}

/// Convert a single `FQGenericArgument` into a `GenericArgument`.
fn resolve_fq_generic_arg(
    arg: &FQGenericArgument,
    krate_collection: &CrateCollection,
) -> Result<GenericArgument, anyhow::Error> {
    match arg {
        FQGenericArgument::Type(t) => Ok(GenericArgument::TypeParameter(resolve_fq_path_type(
            t,
            krate_collection,
        )?)),
        FQGenericArgument::Lifetime(l) => Ok(GenericArgument::Lifetime(l.clone().into())),
    }
}

/// Convert a slice of `FQGenericArgument`s into a `Vec<GenericArgument>`.
fn resolve_fq_generic_args(
    args: &[FQGenericArgument],
    krate_collection: &CrateCollection,
) -> Result<Vec<GenericArgument>, anyhow::Error> {
    args.iter()
        .map(|arg| resolve_fq_generic_arg(arg, krate_collection))
        .collect()
}

/// Internal helper to carry the resolved path before building the full `Callable`.
enum ResolvedPath {
    FreeFunction(FreeFunctionPath),
    InherentMethod(InherentMethodPath),
    TraitMethod(TraitMethodPath),
}

/// Convert a canonical `FQPath` and the resolved `CallableItem` into a structured path.
fn fq_path_to_resolved_path(
    canonical_path: &FQPath,
    callable_items: &CallableItem,
    krate_collection: &CrateCollection,
) -> Result<ResolvedPath, anyhow::Error> {
    let crate_name = canonical_path.crate_name().to_owned();
    let package_id = canonical_path.package_id.clone();

    let result = match callable_items {
        CallableItem::Function(..) => {
            let n = canonical_path.segments.len();
            let last = &canonical_path.segments[n - 1];
            let module_path = canonical_path.segments[1..n - 1]
                .iter()
                .map(|s| s.ident.clone())
                .collect();
            ResolvedPath::FreeFunction(FreeFunctionPath {
                package_id,
                crate_name,
                module_path,
                function_name: last.ident.clone(),
                function_generics: resolve_fq_generic_args(
                    &last.generic_arguments,
                    krate_collection,
                )?,
            })
        }
        CallableItem::Method {
            qualified_self: Some(_),
            method_owner,
            ..
        } => {
            let n = canonical_path.segments.len();
            let method_segment = &canonical_path.segments[n - 1];
            let trait_segment = &canonical_path.segments[n - 2];
            let module_path = canonical_path.segments[1..n - 2]
                .iter()
                .map(|s| s.ident.clone())
                .collect();
            let fq_self_type = canonical_path
                .qualified_self
                .as_ref()
                .map(|q| q.type_.clone())
                .unwrap_or_else(|| {
                    FQPathType::from(Type::Path(PathType {
                        package_id: method_owner.0.item_id.package_id.clone(),
                        rustdoc_id: None,
                        base_type: method_owner
                            .1
                            .segments
                            .iter()
                            .map(|s| s.ident.clone())
                            .collect(),
                        generic_arguments: vec![],
                    }))
                });
            let self_type = resolve_fq_path_type(&fq_self_type, krate_collection)?;
            ResolvedPath::TraitMethod(TraitMethodPath {
                package_id,
                crate_name,
                module_path,
                trait_name: trait_segment.ident.clone(),
                trait_generics: resolve_fq_generic_args(
                    &trait_segment.generic_arguments,
                    krate_collection,
                )?,
                self_type,
                method_name: method_segment.ident.clone(),
                method_generics: resolve_fq_generic_args(
                    &method_segment.generic_arguments,
                    krate_collection,
                )?,
            })
        }
        CallableItem::Method { .. } => {
            let n = canonical_path.segments.len();
            let method_segment = &canonical_path.segments[n - 1];
            let type_segment = &canonical_path.segments[n - 2];
            let module_path = canonical_path.segments[1..n - 2]
                .iter()
                .map(|s| s.ident.clone())
                .collect();
            ResolvedPath::InherentMethod(InherentMethodPath {
                package_id,
                crate_name,
                module_path,
                type_name: type_segment.ident.clone(),
                type_generics: resolve_fq_generic_args(
                    &type_segment.generic_arguments,
                    krate_collection,
                )?,
                method_name: method_segment.ident.clone(),
                method_generics: resolve_fq_generic_args(
                    &method_segment.generic_arguments,
                    krate_collection,
                )?,
            })
        }
    };
    Ok(result)
}

fn get_trait_generic_bindings(
    resolved_item: &ResolvedItem,
    path: &FQPath,
    krate_collection: &CrateCollection,
    generic_bindings: &mut GenericBindings,
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
        if let FQGenericArgument::Type(t) = assigned_parameter {
            // TODO: handle conflicts
            generic_bindings.types.insert(
                generic_slot.name.clone(),
                resolve_fq_path_type(t, krate_collection)?,
            );
        }
    }
    Ok(())
}

pub(crate) fn resolve_type_path(
    path: &FQPath,
    krate_collection: &CrateCollection,
) -> Result<Type, TypePathResolutionError> {
    fn _resolve_type_path(
        path: &FQPath,
        krate_collection: &CrateCollection,
    ) -> Result<Type, anyhow::Error> {
        let item = find_rustdoc_item_type(path, krate_collection)?.1;
        resolve_type_path_with_item(path, &item, krate_collection)
    }

    _resolve_type_path(path, krate_collection).map_err(|source| TypePathResolutionError {
        path: path.clone(),
        source,
    })
}

pub(crate) fn resolve_type_path_with_item(
    path: &FQPath,
    resolved_item: &ResolvedItem,
    krate_collection: &CrateCollection,
) -> Result<Type, anyhow::Error> {
    let item = &resolved_item.item;
    let used_by_package_id = resolved_item.item_id.package_id();
    let (global_type_id, base_type) =
        krate_collection.get_canonical_path_by_local_type_id(used_by_package_id, &item.id, None)?;
    let mut generic_arguments = vec![];
    let (last_segment, first_segments) = path.segments.split_last().unwrap();
    for segment in first_segments {
        for generic_path in &segment.generic_arguments {
            let arg = match generic_path {
                FQGenericArgument::Type(t) => {
                    GenericArgument::TypeParameter(resolve_fq_path_type(t, krate_collection)?)
                }
                FQGenericArgument::Lifetime(l) => GenericArgument::Lifetime(l.clone().into()),
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
                FQGenericArgument::Type(t) => {
                    GenericArgument::TypeParameter(resolve_fq_path_type(t, krate_collection)?)
                }
                FQGenericArgument::Lifetime(l) => GenericArgument::Lifetime(l.clone().into()),
            }
        } else {
            match &generic_def.kind {
                GenericParamDefKind::Lifetime { .. } => GenericArgument::Lifetime(
                    GenericLifetimeParameter::from_name(&generic_def.name),
                ),
                GenericParamDefKind::Type { default, .. } => {
                    if let Some(default) = default {
                        let default = resolve_type(
                            default,
                            used_by_package_id,
                            krate_collection,
                            &GenericBindings::default(),
                        )?;
                        if skip_default(krate_collection, &default) {
                            continue;
                        }
                        GenericArgument::TypeParameter(default)
                    } else {
                        GenericArgument::TypeParameter(Type::Generic(Generic {
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

/// This is a gigantic hack to work around an issue with `std`'s collections:
/// they are all generic over the allocator type, but the default (`alloc::alloc::Global`)
/// is a nightly-only type.
/// If you spell it out, the code won't compile on stable, even though it does
/// exactly the same thing as omitting the parameter.
fn skip_default(krate_collection: &CrateCollection, default: &Type) -> bool {
    use once_cell::sync::OnceCell;
    static GLOBAL_ALLOCATOR: OnceCell<Type> = OnceCell::new();

    let alloc = GLOBAL_ALLOCATOR
        .get_or_init(|| super::utils::resolve_type_path("alloc::alloc::Global", krate_collection));
    alloc == default
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
    SelfResolutionError(#[from] SelfResolutionError),
    #[error(transparent)]
    InputParameterResolutionError(#[from] InputParameterResolutionError),
    #[error(transparent)]
    OutputTypeResolutionError(#[from] OutputTypeResolutionError),
    #[error(transparent)]
    CannotGetCrateData(#[from] CannotGetCrateData),
}

impl From<rustdoc_resolver::CallableResolutionError> for CallableResolutionError {
    fn from(e: rustdoc_resolver::CallableResolutionError) -> Self {
        match e {
            rustdoc_resolver::CallableResolutionError::SelfResolutionError(e) => {
                CallableResolutionError::SelfResolutionError(e)
            }
            rustdoc_resolver::CallableResolutionError::InputParameterResolutionError(e) => {
                CallableResolutionError::InputParameterResolutionError(e)
            }
            rustdoc_resolver::CallableResolutionError::OutputTypeResolutionError(e) => {
                CallableResolutionError::OutputTypeResolutionError(e)
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("I can't resolve `{path}` to a type.")]
pub(crate) struct TypePathResolutionError {
    path: FQPath,
    #[source]
    pub source: anyhow::Error,
}

#[derive(Debug, thiserror::Error, Clone)]
#[error(
    "I can work with functions and methods, but `{import_path}` is neither.\nIt is {item_kind} and I don't know how to handle it here."
)]
pub(crate) struct UnsupportedCallableKind {
    pub import_path: FQPath,
    pub item_kind: String,
}

#[derive(Debug, thiserror::Error, Clone)]
#[error(
    "I can't handle `{generic_type}`, one of the generic parameters you specified for `{callable_path}`."
)]
pub(crate) struct GenericParameterResolutionError {
    pub callable_path: FQPath,
    pub callable_item: rustdoc_types::Item,
    pub generic_type: FQPathType,
    #[source]
    pub source: Arc<anyhow::Error>,
}
