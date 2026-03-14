//! Core type resolution logic: converting `rustdoc_types::Type` into `rustdoc_ir::Type`.

use std::ops::Deref;

use guppy::PackageId;
use once_cell::sync::OnceCell;
use rustdoc_types::{GenericArg, GenericArgs, GenericParamDefKind, ItemEnum, Type as RustdocType};

use rustdoc_ir::{
    Array, FunctionPointer, Generic, GenericArgument, GenericLifetimeParameter, PathType,
    RawPointer, Slice, Tuple, Type, TypeReference,
};
use rustdoc_processor::CrateCollection;
use rustdoc_processor::indexing::CrateIndexer;

use crate::GenericBindings;
use crate::errors::*;

/// Convert a `rustdoc_types::Type` into a `rustdoc_ir::Type`, recursively resolving
/// through type aliases and substituting generic bindings.
pub fn resolve_type<I: CrateIndexer>(
    type_: &RustdocType,
    // The package id where the type we are trying to process has been referenced (e.g. as an
    // input/output parameter).
    used_by_package_id: &PackageId,
    krate_collection: &CrateCollection<I>,
    generic_bindings: &GenericBindings,
) -> Result<Type, TypeResolutionError> {
    _resolve_type(
        type_,
        used_by_package_id,
        krate_collection,
        generic_bindings,
    )
    .map_err(|details| TypeResolutionError {
        ty: type_.to_owned(),
        details,
    })
}

fn _resolve_type<I: CrateIndexer>(
    type_: &RustdocType,
    // The package id where the type we are trying to process has been referenced (e.g. as an
    // input/output parameter).
    used_by_package_id: &PackageId,
    krate_collection: &CrateCollection<I>,
    generic_bindings: &GenericBindings,
) -> Result<Type, TypeResolutionErrorDetails> {
    match type_ {
        RustdocType::ResolvedPath(rustdoc_types::Path {
            id,
            args,
            path: name,
        }) => {
            let re_exporter_crate_name = {
                let mut re_exporter = None;
                if let Some(krate) = krate_collection.get_crate_by_package_id(used_by_package_id)
                    && let Some(item) = krate.maybe_get_item_by_local_type_id(id)
                {
                    // 0 is the crate index of local types.
                    if item.crate_id == 0 {
                        re_exporter = Some(None);
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
                .get_canonical_path_by_local_type_id(used_by_package_id, id, re_exporter_crate_name)
                .map_err(TypeResolutionErrorDetails::ItemResolutionError)?;
            let type_item = krate_collection.get_item_by_global_type_id(&global_type_id);
            // We want to remove any indirections (e.g. `type Foo = Bar;`) and get the actual type.
            if let ItemEnum::TypeAlias(type_alias) = &type_item.inner {
                let mut alias_generic_bindings = GenericBindings::default();
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
                    match &generic_param_def.kind {
                        GenericParamDefKind::Type { default, .. } => {
                            let provided_arg = generic_args.and_then(|v| v.get(i));
                            let generic_type = if let Some(provided_arg) = provided_arg {
                                if let GenericArg::Type(provided_arg) = provided_arg {
                                    resolve_type(
                                        provided_arg,
                                        used_by_package_id,
                                        krate_collection,
                                        generic_bindings,
                                    )
                                    .map_err(|source| {
                                        TypeResolutionErrorDetails::TypePartResolutionError(
                                            Box::new(TypePartResolutionError {
                                                role: format!(
                                                    "generic argument ({})",
                                                    generic_param_def.name
                                                ),
                                                source,
                                            }),
                                        )
                                    })?
                                } else {
                                    let found = match provided_arg {
                                        GenericArg::Lifetime(_) => "lifetime",
                                        GenericArg::Type(_) => "type",
                                        GenericArg::Const(_) => "constant",
                                        GenericArg::Infer => "inferred",
                                    };
                                    return Err(TypeResolutionErrorDetails::GenericKindMismatch(
                                        GenericKindMismatch {
                                            expected_kind: "type",
                                            parameter_name: generic_param_def.name.to_owned(),
                                            found_kind: found,
                                        },
                                    ));
                                }
                            } else if let Some(default) = default {
                                let default = resolve_type(
                                    default,
                                    &global_type_id.package_id,
                                    krate_collection,
                                    generic_bindings,
                                )
                                .map_err(|source| {
                                    TypeResolutionErrorDetails::TypePartResolutionError(Box::new(
                                        TypePartResolutionError {
                                            role: format!(
                                                "default type for generic argument ({})",
                                                generic_param_def.name
                                            ),
                                            source,
                                        },
                                    ))
                                })?;

                                if skip_default(krate_collection, &default) {
                                    continue;
                                }
                                default
                            } else {
                                Type::Generic(Generic {
                                    name: generic_param_def.name.clone(),
                                })
                            };
                            alias_generic_bindings
                                .types
                                .insert(generic_param_def.name.to_string(), generic_type);
                        }
                        GenericParamDefKind::Lifetime { .. } => {
                            let provided_arg = generic_args.and_then(|v| v.get(i));
                            let lifetime = if let Some(provided_arg) = provided_arg {
                                if let GenericArg::Lifetime(provided_arg) = provided_arg {
                                    provided_arg
                                } else {
                                    let found = match provided_arg {
                                        GenericArg::Lifetime(_) => "lifetime",
                                        GenericArg::Type(_) => "type",
                                        GenericArg::Const(_) => "constant",
                                        GenericArg::Infer => "inferred",
                                    };
                                    return Err(TypeResolutionErrorDetails::GenericKindMismatch(
                                        GenericKindMismatch {
                                            expected_kind: "lifetime",
                                            parameter_name: generic_param_def.name.to_owned(),
                                            found_kind: found,
                                        },
                                    ));
                                }
                            } else {
                                &generic_param_def.name
                            }
                            .to_owned();
                            alias_generic_bindings
                                .lifetimes
                                .insert(generic_param_def.name.to_string(), lifetime);
                        }
                        GenericParamDefKind::Const { .. } => {
                            return Err(TypeResolutionErrorDetails::UnsupportedConstGeneric(
                                UnsupportedConstGeneric {
                                    name: generic_param_def.name.to_owned(),
                                },
                            ));
                        }
                    }
                }
                let type_ = resolve_type(
                    &type_alias.type_,
                    &global_type_id.package_id,
                    krate_collection,
                    &alias_generic_bindings,
                )
                .map_err(|source| {
                    TypeResolutionErrorDetails::TypePartResolutionError(Box::new(
                        TypePartResolutionError {
                            role: "aliased type".into(),
                            source,
                        },
                    ))
                })?;

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
                                ItemEnum::Union(u) => &u.generics,
                                i => {
                                    unimplemented!(
                                        "I don't know how to handle a `{:?}` yet, sorry!",
                                        i
                                    )
                                }
                            }
                            .params
                            .as_slice();
                            for (i, arg_def) in generic_arg_defs.iter().enumerate() {
                                let generic_argument = match &arg_def.kind {
                                    GenericParamDefKind::Lifetime { .. } => {
                                        let mut lifetime_name = arg_def.name.clone();
                                        if let Some(GenericArg::Lifetime(l)) = args.get(i) {
                                            lifetime_name = l.clone();
                                        }
                                        GenericArgument::Lifetime(
                                            GenericLifetimeParameter::from_name(lifetime_name),
                                        )
                                    }
                                    GenericParamDefKind::Type { default, .. } => {
                                        if let Some(GenericArg::Type(generic_type)) = args.get(i) {
                                            if let RustdocType::Generic(generic) = generic_type {
                                                if let Some(resolved_type) =
                                                    generic_bindings.types.get(generic)
                                                {
                                                    GenericArgument::TypeParameter(
                                                        resolved_type.to_owned(),
                                                    )
                                                } else {
                                                    GenericArgument::TypeParameter(Type::Generic(
                                                        Generic {
                                                            name: generic.to_owned(),
                                                        },
                                                    ))
                                                }
                                            } else {
                                                GenericArgument::TypeParameter(resolve_type(
                                                    generic_type,
                                                    used_by_package_id,
                                                    krate_collection,
                                                    generic_bindings,
                                                )
                                                .map_err(|source| {
                                                    TypeResolutionErrorDetails::TypePartResolutionError(
                                                        Box::new(TypePartResolutionError {
                                                            role: format!(
                                                                "assigned type for generic parameter `{}`",
                                                                arg_def.name
                                                            ),
                                                            source,
                                                        }),
                                                    )
                                                })?)
                                            }
                                        } else if let Some(default) = default {
                                            let default = resolve_type(
                                                default,
                                                &global_type_id.package_id,
                                                krate_collection,
                                                generic_bindings,
                                            )
                                            .map_err(|source| {
                                                TypeResolutionErrorDetails::TypePartResolutionError(
                                                    Box::new(TypePartResolutionError {
                                                        role: format!(
                                                            "default type for generic parameter `{}`",
                                                            arg_def.name
                                                        ),
                                                        source,
                                                    }),
                                                )
                                            })?;
                                            if skip_default(krate_collection, &default) {
                                                continue;
                                            }
                                            GenericArgument::TypeParameter(default)
                                        } else {
                                            GenericArgument::TypeParameter(Type::Generic(Generic {
                                                name: arg_def.name.clone(),
                                            }))
                                        }
                                    }
                                    GenericParamDefKind::Const { .. } => {
                                        return Err(
                                            TypeResolutionErrorDetails::UnsupportedConstGeneric(
                                                UnsupportedConstGeneric {
                                                    name: arg_def.name.to_owned(),
                                                },
                                            ),
                                        );
                                    }
                                };
                                generics.push(generic_argument);
                            }
                        }
                        GenericArgs::Parenthesized { inputs, output } => {
                            return Err(TypeResolutionErrorDetails::UnsupportedFnPointer(
                                UnsupportedFnPointer {
                                    inputs: inputs.to_owned(),
                                    output: output.to_owned(),
                                },
                            ));
                        }
                        GenericArgs::ReturnTypeNotation => {
                            return Err(TypeResolutionErrorDetails::UnsupportedReturnTypeNotation);
                        }
                    }
                }
                let t = PathType {
                    package_id: global_type_id.package_id().to_owned(),
                    rustdoc_id: Some(global_type_id.rustdoc_item_id),
                    base_type: base_type.to_vec(),
                    generic_arguments: generics,
                };
                Ok(Type::Path(t))
            }
        }
        RustdocType::BorrowedRef {
            lifetime,
            type_,
            is_mutable,
        } => {
            let resolved_type = resolve_type(
                type_,
                used_by_package_id,
                krate_collection,
                generic_bindings,
            )
            .map_err(|source| {
                TypeResolutionErrorDetails::TypePartResolutionError(Box::new(
                    TypePartResolutionError {
                        role: "referenced type".into(),
                        source,
                    },
                ))
            })?;
            let t = TypeReference {
                is_mutable: *is_mutable,
                lifetime: lifetime.to_owned().into(),
                inner: Box::new(resolved_type),
            };
            Ok(t.into())
        }
        RustdocType::Generic(s) => {
            if let Some(resolved_type) = generic_bindings.types.get(s) {
                Ok(resolved_type.to_owned())
            } else {
                Ok(Type::Generic(Generic { name: s.to_owned() }))
            }
        }
        RustdocType::Tuple(t) => {
            let mut types = Vec::with_capacity(t.len());
            for (i, type_) in t.iter().enumerate() {
                let type_ = resolve_type(
                    type_,
                    used_by_package_id,
                    krate_collection,
                    generic_bindings,
                )
                .map_err(|source| {
                    TypeResolutionErrorDetails::TypePartResolutionError(Box::new(
                        TypePartResolutionError {
                            role: format!("type of element {} in tuple", i + 1),
                            source,
                        },
                    ))
                })?;
                types.push(type_);
            }
            Ok(Type::Tuple(Tuple { elements: types }))
        }
        RustdocType::Primitive(p) => Ok(Type::ScalarPrimitive(
            p.as_str()
                .try_into()
                .map_err(TypeResolutionErrorDetails::UnknownPrimitive)?,
        )),
        RustdocType::Slice(type_) => {
            let inner = resolve_type(
                type_,
                used_by_package_id,
                krate_collection,
                generic_bindings,
            )
            .map_err(|source| {
                TypeResolutionErrorDetails::TypePartResolutionError(Box::new(
                    TypePartResolutionError {
                        role: "slice type".into(),
                        source,
                    },
                ))
            })?;

            Ok(Type::Slice(Slice {
                element_type: Box::new(inner),
            }))
        }
        RustdocType::Array { type_, len } => {
            let len: usize = len.parse().map_err(|_| {
                TypeResolutionErrorDetails::UnsupportedArrayLength(UnsupportedArrayLength {
                    len: len.clone(),
                })
            })?;
            let resolved = resolve_type(
                type_,
                used_by_package_id,
                krate_collection,
                generic_bindings,
            )
            .map_err(|source| {
                TypeResolutionErrorDetails::TypePartResolutionError(Box::new(
                    TypePartResolutionError {
                        role: "array element type".into(),
                        source,
                    },
                ))
            })?;
            Ok(Type::Array(Array {
                element_type: Box::new(resolved),
                len,
            }))
        }
        RustdocType::DynTrait(_) => Err(TypeResolutionErrorDetails::UnsupportedTypeKind(
            UnsupportedTypeKind { kind: "dyn trait" },
        )),
        RustdocType::FunctionPointer(fp) => {
            if !fp.generic_params.is_empty() {
                return Err(TypeResolutionErrorDetails::UnsupportedTypeKind(
                    UnsupportedTypeKind {
                        kind: "higher-ranked function pointer",
                    },
                ));
            }

            let inputs = fp
                .sig
                .inputs
                .iter()
                .enumerate()
                .map(|(i, (_, ty))| {
                    resolve_type(ty, used_by_package_id, krate_collection, generic_bindings)
                        .map_err(|source| {
                            TypeResolutionErrorDetails::TypePartResolutionError(Box::new(
                                TypePartResolutionError {
                                    role: format!("function pointer input {}", i),
                                    source,
                                },
                            ))
                        })
                })
                .collect::<Result<Vec<_>, _>>()?;

            let output = fp
                .sig
                .output
                .as_ref()
                .map(|ty| {
                    resolve_type(ty, used_by_package_id, krate_collection, generic_bindings)
                        .map_err(|source| {
                            TypeResolutionErrorDetails::TypePartResolutionError(Box::new(
                                TypePartResolutionError {
                                    role: "function pointer output".into(),
                                    source,
                                },
                            ))
                        })
                })
                .transpose()?;

            Ok(Type::FunctionPointer(FunctionPointer {
                inputs,
                output: output.map(Box::new),
                abi: fp.header.abi.clone(),
                is_unsafe: fp.header.is_unsafe,
            }))
        }
        RustdocType::Pat { .. } => Err(TypeResolutionErrorDetails::UnsupportedTypeKind(
            UnsupportedTypeKind { kind: "pattern" },
        )),
        RustdocType::ImplTrait(..) => Err(TypeResolutionErrorDetails::UnsupportedTypeKind(
            UnsupportedTypeKind { kind: "impl trait" },
        )),
        RustdocType::Infer => Err(TypeResolutionErrorDetails::UnsupportedTypeKind(
            UnsupportedTypeKind {
                kind: "inferred type",
            },
        )),
        RustdocType::RawPointer { is_mutable, type_ } => {
            let resolved = resolve_type(
                type_,
                used_by_package_id,
                krate_collection,
                generic_bindings,
            )
            .map_err(|source| {
                TypeResolutionErrorDetails::TypePartResolutionError(Box::new(
                    TypePartResolutionError {
                        role: "pointee type".into(),
                        source,
                    },
                ))
            })?;
            Ok(Type::RawPointer(RawPointer {
                is_mutable: *is_mutable,
                inner: Box::new(resolved),
            }))
        }
        RustdocType::QualifiedPath { .. } => Err(TypeResolutionErrorDetails::UnsupportedTypeKind(
            UnsupportedTypeKind {
                kind: "qualified path",
            },
        )),
    }
}

/// This is a gigantic hack to work around an issue with `std`'s collections:
/// they are all generic over the allocator type, but the default (`alloc::alloc::Global`)
/// is a nightly-only type.
/// If you spell it out, the code won't compile on stable, even though it does
/// exactly the same thing as omitting the parameter.
fn skip_default<I: CrateIndexer>(krate_collection: &CrateCollection<I>, default: &Type) -> bool {
    static GLOBAL_ALLOCATOR_PATH: OnceCell<Vec<String>> = OnceCell::new();

    let Type::Path(path_type) = default else {
        return false;
    };

    let expected_path = GLOBAL_ALLOCATOR_PATH.get_or_init(|| {
        // Try to resolve `alloc::alloc::Global` via the collection to get
        // the canonical path.  Fall back to the well-known path if resolution
        // fails (e.g. alloc docs not available).
        let alloc_pid = PackageId::new(rustdoc_processor::ALLOC_PACKAGE_ID_REPR);
        if let Some(krate) = krate_collection.get_crate_by_package_id(&alloc_pid)
            && let Some(item_entry) = krate
                .import_index
                .items
                .iter()
                .find(|(_, entry)| entry.canonical_path() == ["alloc", "alloc", "Global"])
        {
            return item_entry.1.canonical_path().to_vec();
        }
        vec!["alloc".into(), "alloc".into(), "Global".into()]
    });

    path_type.base_type == *expected_path
}
