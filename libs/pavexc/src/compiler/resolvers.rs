//! Given the fully qualified path to a function (be it a constructor or a handler),
//! find the corresponding item ("resolution") in `rustdoc`'s JSON output to determine
//! its input parameters and output type.
use std::ops::Deref;
use std::sync::Arc;

use ahash::HashMap;
use guppy::PackageId;
use once_cell::sync::OnceCell;
use rustdoc_types::{GenericArg, GenericArgs, GenericParamDefKind, ItemEnum, Type};
use tracing_log_error::log_error;

use crate::language::{
    Callable, CallableItem, FQGenericArgument, FQPath, FQPathSegment, FQPathType, Generic,
    GenericArgument, GenericLifetimeParameter, InvocationStyle, PathType, ResolvedPathLifetime,
    ResolvedType, Slice, Tuple, TypeReference, UnknownPath, UnknownPrimitive,
};
use crate::rustdoc::{CannotGetCrateData, RustdocKindExt};
use crate::rustdoc::{CrateCollection, ResolvedItem};

#[derive(Default)]
pub(crate) struct GenericBindings {
    pub lifetimes: HashMap<String, String>,
    pub types: HashMap<String, ResolvedType>,
}

impl std::fmt::Debug for GenericBindings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GenericBindings {{ ")?;
        if !self.lifetimes.is_empty() {
            write!(f, "lifetimes: {{ ")?;
            for (name, value) in &self.lifetimes {
                writeln!(f, "{} -> {}, ", name, value)?;
            }
            write!(f, "}}, ")?;
        }
        if !self.types.is_empty() {
            write!(f, "types: {{ ")?;
            for (name, value) in &self.types {
                writeln!(f, "{} -> {}, ", name, value.display_for_error())?;
            }
            write!(f, "}}, ")?;
        }
        write!(f, "}}")
    }
}

#[derive(Debug)]
pub struct TypeResolutionError {
    pub ty: Type,
    pub details: TypeResolutionErrorDetails,
}

impl std::fmt::Display for TypeResolutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Failed to resolve a type, {:?}.", self.ty)?;
        match &self.details {
            TypeResolutionErrorDetails::UnsupportedConstGeneric(unsupported_const_generic) => {
                write!(
                    f,
                    "It uses a const generic parameter, {}, which isn't currently supported.",
                    &unsupported_const_generic.name
                )
            }
            TypeResolutionErrorDetails::UnsupportedFnPointer(unsupported_fn_pointer) => {
                write!(
                    f,
                    "It uses a function pointer with inputs {:?} and output {:?}, which isn't currently supported.",
                    unsupported_fn_pointer.inputs, unsupported_fn_pointer.output
                )
            }
            TypeResolutionErrorDetails::UnsupportedReturnTypeNotation => {
                write!(
                    f,
                    "It uses return type notation, which isn't currently supported."
                )
            }
            TypeResolutionErrorDetails::UnsupportedTypeKind(unsupported_type_kind) => {
                write!(
                    f,
                    "It is a `{}`, which isn't currently supported.",
                    unsupported_type_kind.kind
                )
            }
            TypeResolutionErrorDetails::GenericKindMismatch(generic_kind_mismatch) => {
                write!(
                    f,
                    "There was a generic kind mismatch: for parameter `{}` we expected kind `{}` but found kind `{}`.",
                    generic_kind_mismatch.parameter_name,
                    generic_kind_mismatch.expected_kind,
                    generic_kind_mismatch.found_kind
                )
            }
            TypeResolutionErrorDetails::ItemResolutionError(_) => {
                write!(f, "We failed to resolve one of its sub-types.")
            }
            TypeResolutionErrorDetails::TypePartResolutionError(part) => {
                write!(f, "We failed to resolve {}", part.role)
            }
            TypeResolutionErrorDetails::UnknownPrimitive(e) => {
                write!(f, "{e}")
            }
        }
    }
}

impl std::error::Error for TypeResolutionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.details {
            TypeResolutionErrorDetails::ItemResolutionError(err) => Some(err.deref()),
            TypeResolutionErrorDetails::TypePartResolutionError(err) => Some(&err.source),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum TypeResolutionErrorDetails {
    UnsupportedConstGeneric(UnsupportedConstGeneric),
    UnsupportedFnPointer(UnsupportedFnPointer),
    UnsupportedReturnTypeNotation,
    UnsupportedTypeKind(UnsupportedTypeKind),
    UnknownPrimitive(UnknownPrimitive),
    GenericKindMismatch(GenericKindMismatch),
    ItemResolutionError(anyhow::Error),
    TypePartResolutionError(Box<TypePartResolutionError>),
}

#[derive(Debug)]
pub struct UnsupportedConstGeneric {
    pub name: String,
}

#[derive(Debug)]
pub struct UnsupportedFnPointer {
    /// The input types, enclosed in parentheses.
    pub inputs: Vec<Type>,
    /// The output type provided after the `->`, if present.
    pub output: Option<Type>,
}

#[derive(Debug)]
pub struct TypePartResolutionError {
    pub role: String,
    pub source: TypeResolutionError,
}

#[derive(Debug)]
pub struct UnsupportedTypeKind {
    pub kind: &'static str,
}

#[derive(Debug)]
pub struct GenericKindMismatch {
    pub parameter_name: String,
    pub expected_kind: &'static str,
    pub found_kind: &'static str,
}

pub(crate) fn resolve_type(
    type_: &Type,
    // The package id where the type we are trying to process has been referenced (e.g. as an
    // input/output parameter).
    used_by_package_id: &PackageId,
    krate_collection: &CrateCollection,
    generic_bindings: &GenericBindings,
) -> Result<ResolvedType, TypeResolutionError> {
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

pub(crate) fn _resolve_type(
    type_: &Type,
    // The package id where the type we are trying to process has been referenced (e.g. as an
    // input/output parameter).
    used_by_package_id: &PackageId,
    krate_collection: &CrateCollection,
    generic_bindings: &GenericBindings,
) -> Result<ResolvedType, TypeResolutionErrorDetails> {
    match type_ {
        Type::ResolvedPath(rustdoc_types::Path {
            id,
            args,
            path: name,
        }) => {
            let re_exporter_crate_name = {
                let mut re_exporter = None;
                if let Some(krate) = krate_collection.get_crate_by_package_id(used_by_package_id) {
                    if let Some(item) = krate.maybe_get_item_by_local_type_id(id) {
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
                .get_canonical_path_by_local_type_id(used_by_package_id, id, re_exporter_crate_name)
                .map_err(|e| TypeResolutionErrorDetails::ItemResolutionError(e))?;
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
                                            expected_kind: "type".into(),
                                            parameter_name: generic_param_def.name.to_owned(),
                                            found_kind: found.into(),
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
                                ResolvedType::Generic(Generic {
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
                                            expected_kind: "lifetime".into(),
                                            parameter_name: generic_param_def.name.to_owned(),
                                            found_kind: found.into(),
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
                                                    generic_bindings.types.get(generic)
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
                                            GenericArgument::TypeParameter(ResolvedType::Generic(
                                                Generic {
                                                    name: arg_def.name.clone(),
                                                },
                                            ))
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
        Type::Generic(s) => {
            if let Some(resolved_type) = generic_bindings.types.get(s) {
                Ok(resolved_type.to_owned())
            } else {
                Ok(ResolvedType::Generic(Generic { name: s.to_owned() }))
            }
        }
        Type::Tuple(t) => {
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
            Ok(ResolvedType::Tuple(Tuple { elements: types }))
        }
        Type::Primitive(p) => Ok(ResolvedType::ScalarPrimitive(
            p.as_str()
                .try_into()
                .map_err(|e| TypeResolutionErrorDetails::UnknownPrimitive(e))?,
        )),
        Type::Slice(type_) => {
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

            Ok(ResolvedType::Slice(Slice {
                element_type: Box::new(inner),
            }))
        }
        Type::Array { .. } => Err(TypeResolutionErrorDetails::UnsupportedTypeKind(
            UnsupportedTypeKind { kind: "array" },
        )),
        Type::DynTrait(_) => Err(TypeResolutionErrorDetails::UnsupportedTypeKind(
            UnsupportedTypeKind { kind: "dyn trait" },
        )),
        Type::FunctionPointer(_) => Err(TypeResolutionErrorDetails::UnsupportedTypeKind(
            UnsupportedTypeKind {
                kind: "function pointer",
            },
        )),
        Type::Pat { .. } => Err(TypeResolutionErrorDetails::UnsupportedTypeKind(
            UnsupportedTypeKind { kind: "pattern" },
        )),
        Type::ImplTrait(..) => Err(TypeResolutionErrorDetails::UnsupportedTypeKind(
            UnsupportedTypeKind { kind: "impl trait" },
        )),
        Type::Infer => Err(TypeResolutionErrorDetails::UnsupportedTypeKind(
            UnsupportedTypeKind {
                kind: "inferred type",
            },
        )),
        Type::RawPointer { .. } => Err(TypeResolutionErrorDetails::UnsupportedTypeKind(
            UnsupportedTypeKind {
                kind: "raw pointer",
            },
        )),
        Type::QualifiedPath { .. } => Err(TypeResolutionErrorDetails::UnsupportedTypeKind(
            UnsupportedTypeKind {
                kind: "qualified path",
            },
        )),
    }
}

pub(crate) fn resolve_callable(
    krate_collection: &CrateCollection,
    callable_path: &FQPath,
) -> Result<Callable, CallableResolutionError> {
    let callable_items = callable_path.find_rustdoc_callable_items(krate_collection)??;
    let (callable_item, new_callable_path) = match &callable_items {
        CallableItem::Function(item, p) => (item, p),
        CallableItem::Method { method, .. } => (&method.0, &method.1),
    };
    let used_by_package_id = &new_callable_path.package_id;

    let (header, decl, fn_generics_defs, invocation_style) = match &callable_item.item.inner {
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

    let mut generic_bindings = GenericBindings::default();
    if let CallableItem::Method {
        method_owner,
        qualified_self,
        ..
    } = &callable_items
    {
        if matches!(&method_owner.0.item.inner, ItemEnum::Trait(_)) {
            if let Err(e) = get_trait_generic_bindings(
                &method_owner.0,
                &method_owner.1,
                krate_collection,
                &mut generic_bindings,
            ) {
                log_error!(*e, level: tracing::Level::WARN, "Error getting trait generic bindings");
            }
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
                let resolved_type =
                    t.resolve(krate_collection)
                        .map_err(|e| GenericParameterResolutionError {
                            generic_type: t.to_owned(),
                            callable_path: new_callable_path.to_owned(),
                            callable_item: callable_item.item.clone().into_owned(),
                            source: Arc::new(e),
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
                    callable_path: new_callable_path.to_owned(),
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
                        callable_path: new_callable_path.to_owned(),
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

        let canonical_path = match parent_canonical_path {
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
        source_coordinates: Some(callable_item.item_id.clone()),
    };
    Ok(callable)
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
            generic_bindings
                .types
                .insert(generic_slot.name.clone(), t.resolve(krate_collection)?);
        }
    }
    Ok(())
}

pub(crate) fn resolve_type_path(
    path: &FQPath,
    krate_collection: &CrateCollection,
) -> Result<ResolvedType, TypePathResolutionError> {
    fn _resolve_type_path(
        path: &FQPath,
        krate_collection: &CrateCollection,
    ) -> Result<ResolvedType, anyhow::Error> {
        let item = path.find_rustdoc_item_type(krate_collection)?.1;
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
                FQGenericArgument::Type(t) => {
                    GenericArgument::TypeParameter(t.resolve(krate_collection)?)
                }
                FQGenericArgument::Lifetime(l) => match l {
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
                FQGenericArgument::Type(t) => {
                    GenericArgument::TypeParameter(t.resolve(krate_collection)?)
                }
                FQGenericArgument::Lifetime(l) => match l {
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
                            &GenericBindings::default(),
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
    SelfResolutionError(#[from] SelfResolutionError),
    #[error(transparent)]
    InputParameterResolutionError(#[from] InputParameterResolutionError),
    #[error(transparent)]
    OutputTypeResolutionError(#[from] OutputTypeResolutionError),
    #[error(transparent)]
    CannotGetCrateData(#[from] CannotGetCrateData),
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
#[error("One of the input parameters for `{callable_path}` has a type that I can't handle.")]
pub(crate) struct InputParameterResolutionError {
    pub callable_path: FQPath,
    pub callable_item: rustdoc_types::Item,
    pub parameter_type: Type,
    pub parameter_index: usize,
    #[source]
    pub source: Arc<anyhow::Error>,
}

#[derive(Debug, thiserror::Error, Clone)]
#[error("I can't handle the `Self` type for `{path}`.")]
pub(crate) struct SelfResolutionError {
    pub path: FQPath,
    #[source]
    pub source: Arc<anyhow::Error>,
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

#[derive(Debug, thiserror::Error, Clone)]
#[error("I don't know how to handle the type returned by `{callable_path}`.")]
pub(crate) struct OutputTypeResolutionError {
    pub callable_path: FQPath,
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

    let alloc = GLOBAL_ALLOCATOR
        .get_or_init(|| super::utils::resolve_type_path("alloc::alloc::Global", krate_collection));
    alloc == default
}
