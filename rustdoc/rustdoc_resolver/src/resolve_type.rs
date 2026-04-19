//! Core type resolution logic: converting `rustdoc_types::Type` into `rustdoc_ir::Type`.

use std::ops::Deref;

use guppy::PackageId;
use once_cell::sync::OnceCell;
use rustdoc_types::{GenericArg, GenericArgs, GenericParamDefKind, ItemEnum, Type as RustdocType};

use rustdoc_ir::{
    Array, ConstGenericArgument, FunctionPointer, FunctionPointerInput, Generic, GenericArgument,
    GenericLifetimeParameter, PathType, RawPointer, Slice, Tuple, Type, TypeReference,
};
use rustdoc_processor::CrateCollection;
use rustdoc_processor::GlobalItemId;
use rustdoc_processor::indexing::CrateIndexer;

use crate::GenericBindings;
use crate::errors::*;

/// Controls whether `resolve_type` resolves through type aliases or preserves them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeAliasResolution {
    /// Resolve through type aliases, returning the underlying type (current default).
    ResolveThrough,
    /// Stop at type aliases, returning `Type::TypeAlias(PathType)`.
    Preserve,
}

/// Convert a `rustdoc_types::Type` into a `rustdoc_ir::Type`, recursively resolving
/// through type aliases and substituting generic bindings.
pub fn resolve_type<I: CrateIndexer>(
    type_: &RustdocType,
    // The package id where the type we are trying to process has been referenced (e.g. as an
    // input/output parameter).
    used_by_package_id: &PackageId,
    krate_collection: &CrateCollection<I>,
    generic_bindings: &GenericBindings,
    alias_resolution: TypeAliasResolution,
) -> Result<Type, TypeResolutionError> {
    _resolve_type(
        type_,
        used_by_package_id,
        krate_collection,
        generic_bindings,
        alias_resolution,
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
    alias_resolution: TypeAliasResolution,
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

                // When preserving, we need to resolve the generic arguments
                // for the alias's own identity, but we don't resolve through.
                let mut resolved_alias_generics = vec![];

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
                                        alias_resolution,
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
                                    alias_resolution,
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
                                .insert(generic_param_def.name.to_string(), generic_type.clone());
                            resolved_alias_generics
                                .push(GenericArgument::TypeParameter(generic_type));
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
                                .insert(generic_param_def.name.to_string(), lifetime.clone());
                            resolved_alias_generics.push(GenericArgument::Lifetime(
                                GenericLifetimeParameter::from_name(lifetime),
                            ));
                        }
                        GenericParamDefKind::Const { default, .. } => {
                            let provided_arg = generic_args.and_then(|v| v.get(i));
                            let value = if let Some(GenericArg::Const(constant)) = provided_arg {
                                let raw = constant.value.as_ref().unwrap_or(&constant.expr);
                                generic_bindings
                                    .consts
                                    .get(raw)
                                    .cloned()
                                    .unwrap_or_else(|| raw.clone())
                            } else if let Some(default) = default {
                                default.clone()
                            } else {
                                generic_param_def.name.clone()
                            };
                            alias_generic_bindings
                                .consts
                                .insert(generic_param_def.name.to_string(), value.clone());
                            resolved_alias_generics
                                .push(GenericArgument::Const(ConstGenericArgument { value }));
                        }
                    }
                }

                if alias_resolution == TypeAliasResolution::Preserve {
                    let alias_path = PathType {
                        package_id: global_type_id.package_id().to_owned(),
                        rustdoc_id: Some(global_type_id.rustdoc_item_id),
                        base_type: base_type.to_vec(),
                        generic_arguments: resolved_alias_generics,
                    };
                    return Ok(Type::TypeAlias(alias_path));
                }

                let type_ = resolve_type(
                    &type_alias.type_,
                    &global_type_id.package_id,
                    krate_collection,
                    &alias_generic_bindings,
                    alias_resolution,
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
                            // Track resolved type parameters so that defaults for
                            // later parameters can reference earlier ones.
                            // E.g. `BitFlags<T, N = <T as RawBitFlags>::Numeric>` —
                            // when resolving N's default, T must already be bound.
                            let mut local_generic_bindings = generic_bindings.clone();
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
                                        let resolved = if let Some(GenericArg::Type(generic_type)) =
                                            args.get(i)
                                        {
                                            if let RustdocType::Generic(generic) = generic_type {
                                                if let Some(resolved_type) =
                                                    generic_bindings.types.get(generic)
                                                {
                                                    resolved_type.to_owned()
                                                } else {
                                                    Type::Generic(Generic {
                                                        name: generic.to_owned(),
                                                    })
                                                }
                                            } else {
                                                resolve_type(
                                                    generic_type,
                                                    used_by_package_id,
                                                    krate_collection,
                                                    generic_bindings,
                                                    alias_resolution,
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
                                                })?
                                            }
                                        } else if let Some(default) = default {
                                            let default = resolve_type(
                                                default,
                                                &global_type_id.package_id,
                                                krate_collection,
                                                &local_generic_bindings,
                                                alias_resolution,
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
                                            default
                                        } else {
                                            Type::Generic(Generic {
                                                name: arg_def.name.clone(),
                                            })
                                        };
                                        local_generic_bindings
                                            .types
                                            .insert(arg_def.name.clone(), resolved.clone());
                                        GenericArgument::TypeParameter(resolved)
                                    }
                                    GenericParamDefKind::Const { default, .. } => {
                                        let value = if let Some(GenericArg::Const(constant)) =
                                            args.get(i)
                                        {
                                            let raw =
                                                constant.value.as_ref().unwrap_or(&constant.expr);
                                            local_generic_bindings
                                                .consts
                                                .get(raw)
                                                .cloned()
                                                .unwrap_or_else(|| raw.clone())
                                        } else if let Some(default) = default {
                                            default.clone()
                                        } else {
                                            arg_def.name.clone()
                                        };
                                        local_generic_bindings
                                            .consts
                                            .insert(arg_def.name.clone(), value.clone());
                                        GenericArgument::Const(ConstGenericArgument { value })
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
                alias_resolution,
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
                    alias_resolution,
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
                alias_resolution,
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
            let resolved_len = generic_bindings
                .consts
                .get(len)
                .map(|s| s.as_str())
                .unwrap_or(len);
            let len: usize = resolved_len.parse().map_err(|_| {
                TypeResolutionErrorDetails::UnsupportedArrayLength(UnsupportedArrayLength {
                    len: len.clone(),
                })
            })?;
            let resolved = resolve_type(
                type_,
                used_by_package_id,
                krate_collection,
                generic_bindings,
                alias_resolution,
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
                .map(|(i, (name, ty))| {
                    let resolved_ty = resolve_type(
                        ty,
                        used_by_package_id,
                        krate_collection,
                        generic_bindings,
                        alias_resolution,
                    )
                    .map_err(|source| {
                        TypeResolutionErrorDetails::TypePartResolutionError(Box::new(
                            TypePartResolutionError {
                                role: format!("function pointer input {}", i),
                                source,
                            },
                        ))
                    })?;
                    let name = match name.as_str() {
                        "" | "_" => None,
                        s => Some(s.to_owned()),
                    };
                    Ok(FunctionPointerInput {
                        name,
                        type_: resolved_ty,
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;

            let output = fp
                .sig
                .output
                .as_ref()
                .map(|ty| {
                    resolve_type(
                        ty,
                        used_by_package_id,
                        krate_collection,
                        generic_bindings,
                        alias_resolution,
                    )
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
                alias_resolution,
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
        RustdocType::QualifiedPath {
            name,
            self_type,
            trait_,
            args: _,
        } => {
            // 1. Resolve self_type to a concrete type
            let resolved_self = resolve_type(
                self_type,
                used_by_package_id,
                krate_collection,
                generic_bindings,
                alias_resolution,
            )
            .map_err(|source| {
                TypeResolutionErrorDetails::TypePartResolutionError(Box::new(
                    TypePartResolutionError {
                        role: "self type of qualified path".into(),
                        source,
                    },
                ))
            })?;

            // 2. Get the trait (if present)
            let Some(trait_path) = trait_ else {
                return Err(TypeResolutionErrorDetails::UnsupportedTypeKind(
                    UnsupportedTypeKind {
                        kind: "inherent associated type",
                    },
                ));
            };

            // 3. Find the associated type in the impl
            resolve_associated_type(
                &resolved_self,
                trait_path,
                name,
                used_by_package_id,
                krate_collection,
                generic_bindings,
                alias_resolution,
            )
        }
    }
}

/// Resolve `<SelfType as Trait>::AssocType` by finding the concrete impl block
/// and looking up the associated type definition.
fn resolve_associated_type<I: CrateIndexer>(
    resolved_self: &Type,
    trait_path: &rustdoc_types::Path,
    assoc_type_name: &str,
    used_by_package_id: &PackageId,
    krate_collection: &CrateCollection<I>,
    generic_bindings: &GenericBindings,
    alias_resolution: TypeAliasResolution,
) -> Result<Type, TypeResolutionErrorDetails> {
    // Resolve the trait path to get its GlobalItemId and canonical path.
    let (trait_global_id, trait_canonical_path) = krate_collection
        .get_canonical_path_by_local_type_id(used_by_package_id, &trait_path.id, None)
        .map_err(TypeResolutionErrorDetails::ItemResolutionError)?;

    // Try to find the associated type in the concrete type's impls first,
    // then fall back to the trait's implementations list.
    if let Some(result) = find_trait_assoc_type_in_type_impls(
        resolved_self,
        trait_canonical_path,
        assoc_type_name,
        krate_collection,
        generic_bindings,
        alias_resolution,
    )? {
        return Ok(result);
    }

    if let Some(result) = find_assoc_type_in_trait_impls(
        resolved_self,
        &trait_global_id,
        assoc_type_name,
        krate_collection,
        generic_bindings,
        alias_resolution,
    )? {
        return Ok(result);
    }

    Err(TypeResolutionErrorDetails::AssociatedTypeResolutionError(
        AssociatedTypeResolutionError {
            assoc_type_name: assoc_type_name.to_owned(),
            trait_path: trait_canonical_path.to_vec(),
            self_type: resolved_self.clone(),
        },
    ))
}

/// Search through the concrete type's `impls` list to find a matching trait impl
/// and extract the associated type.
fn find_trait_assoc_type_in_type_impls<I: CrateIndexer>(
    resolved_self: &Type,
    trait_canonical_path: &[String],
    assoc_type_name: &str,
    krate_collection: &CrateCollection<I>,
    generic_bindings: &GenericBindings,
    alias_resolution: TypeAliasResolution,
) -> Result<Option<Type>, TypeResolutionErrorDetails> {
    // This may not be general enough—i.e. does the self type in a qualified
    // path need to be a `Type::Path`?
    // Without looking too deep into it, my gut feeling is "no", but we
    // can generalize later.
    let Type::Path(self_path) = resolved_self else {
        return Ok(None);
    };

    let type_package_id = &self_path.package_id;
    let Some(type_crate) = krate_collection.get_crate_by_package_id(type_package_id) else {
        return Ok(None);
    };

    // Find the type definition to get its impls list.
    let Some(rustdoc_id) = &self_path.rustdoc_id else {
        return Ok(None);
    };
    let type_item = type_crate.get_item_by_local_type_id(rustdoc_id);
    let impls = match &type_item.inner {
        ItemEnum::Struct(s) => &s.impls,
        ItemEnum::Enum(e) => &e.impls,
        ItemEnum::Union(u) => &u.impls,
        _ => return Ok(None),
    };

    search_impls_for_assoc_type(
        impls,
        type_package_id,
        trait_canonical_path,
        assoc_type_name,
        krate_collection,
        generic_bindings,
        alias_resolution,
    )
}

/// Search through the trait's `implementations` list to find a matching impl
/// and extract the associated type.
fn find_assoc_type_in_trait_impls<I: CrateIndexer>(
    resolved_self: &Type,
    trait_global_id: &GlobalItemId,
    assoc_type_name: &str,
    krate_collection: &CrateCollection<I>,
    generic_bindings: &GenericBindings,
    alias_resolution: TypeAliasResolution,
) -> Result<Option<Type>, TypeResolutionErrorDetails> {
    let trait_item = krate_collection.get_item_by_global_type_id(trait_global_id);
    let ItemEnum::Trait(trait_def) = &trait_item.inner else {
        return Ok(None);
    };

    let resolved_self = resolved_self.canonicalize(krate_collection);

    // The trait's implementations are local to the trait's crate.
    let trait_package_id = &trait_global_id.package_id;
    let Some(trait_crate) = krate_collection.get_crate_by_package_id(trait_package_id) else {
        return Ok(None);
    };
    for impl_id in &trait_def.implementations {
        let Some(impl_item) = trait_crate.maybe_get_item_by_local_type_id(impl_id) else {
            continue;
        };
        let ItemEnum::Impl(impl_) = &impl_item.inner else {
            continue;
        };
        if impl_.is_negative {
            continue;
        }
        // Skip generic/blanket impls — same reasoning as in `search_impls_for_assoc_type`.
        if impl_
            .generics
            .params
            .iter()
            .any(|p| matches!(p.kind, GenericParamDefKind::Type { .. }))
        {
            continue;
        }
        // Try to resolve the impl's `for_` type. If resolution fails, skip.
        let Ok(resolved_for) = resolve_type(
            &impl_.for_,
            trait_package_id,
            krate_collection,
            &GenericBindings::default(),
            alias_resolution,
        ) else {
            continue;
        };
        if resolved_for.canonicalize(krate_collection) != resolved_self {
            continue;
        }

        // Found a matching impl, now look for the associated type.
        if let Some(result) = extract_assoc_type_from_impl_items(
            &impl_.items,
            trait_package_id,
            assoc_type_name,
            krate_collection,
            generic_bindings,
            alias_resolution,
        )? {
            return Ok(Some(result));
        }
    }

    Ok(None)
}

/// Search a list of impl IDs for a matching trait impl and extract the associated type.
fn search_impls_for_assoc_type<I: CrateIndexer>(
    impl_ids: &[rustdoc_types::Id],
    impl_crate_package_id: &PackageId,
    trait_canonical_path: &[String],
    assoc_type_name: &str,
    krate_collection: &CrateCollection<I>,
    generic_bindings: &GenericBindings,
    alias_resolution: TypeAliasResolution,
) -> Result<Option<Type>, TypeResolutionErrorDetails> {
    let Some(impl_crate) = krate_collection.get_crate_by_package_id(impl_crate_package_id) else {
        return Ok(None);
    };

    for impl_id in impl_ids {
        let Some(item) = impl_crate.maybe_get_item_by_local_type_id(impl_id) else {
            continue;
        };
        let ItemEnum::Impl(impl_) = &item.inner else {
            continue;
        };
        if impl_.is_negative {
            continue;
        }
        // Skip generic/blanket impls (e.g., `impl<T> Trait for T` or
        // `impl<T: Clone> Trait for T`). We can't verify trait bounds,
        // and the associated type value may reference the impl's own
        // generic parameters.
        if impl_
            .generics
            .params
            .iter()
            .any(|p| matches!(p.kind, GenericParamDefKind::Type { .. }))
        {
            continue;
        }
        // Check if this impl is for the right trait.
        let Some(impl_trait) = &impl_.trait_ else {
            continue;
        };
        let Ok((_, impl_trait_path)) = krate_collection.get_canonical_path_by_local_type_id(
            impl_crate_package_id,
            &impl_trait.id,
            None,
        ) else {
            continue;
        };
        if impl_trait_path != trait_canonical_path {
            continue;
        }

        // Found a matching trait impl, look for the associated type.
        if let Some(result) = extract_assoc_type_from_impl_items(
            &impl_.items,
            impl_crate_package_id,
            assoc_type_name,
            krate_collection,
            generic_bindings,
            alias_resolution,
        )? {
            return Ok(Some(result));
        }
    }

    Ok(None)
}

/// Extract an associated type by name from an impl block's items.
fn extract_assoc_type_from_impl_items<I: CrateIndexer>(
    items: &[rustdoc_types::Id],
    package_id: &PackageId,
    assoc_type_name: &str,
    krate_collection: &CrateCollection<I>,
    generic_bindings: &GenericBindings,
    alias_resolution: TypeAliasResolution,
) -> Result<Option<Type>, TypeResolutionErrorDetails> {
    let Some(crate_) = krate_collection.get_crate_by_package_id(package_id) else {
        return Ok(None);
    };

    for item_id in items {
        let Some(item) = crate_.maybe_get_item_by_local_type_id(item_id) else {
            continue;
        };
        if item.name.as_deref() != Some(assoc_type_name) {
            continue;
        }
        if let ItemEnum::AssocType {
            type_: Some(concrete_type),
            ..
        } = &item.inner
        {
            // Resolve the concrete type. It's defined in this crate.
            let resolved = resolve_type(
                concrete_type,
                package_id,
                krate_collection,
                generic_bindings,
                alias_resolution,
            )
            .map_err(|source| {
                TypeResolutionErrorDetails::TypePartResolutionError(Box::new(
                    TypePartResolutionError {
                        role: format!("associated type `{}`", assoc_type_name),
                        source,
                    },
                ))
            })?;
            return Ok(Some(resolved));
        }
    }

    Ok(None)
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
