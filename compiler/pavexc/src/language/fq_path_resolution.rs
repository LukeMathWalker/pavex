use std::fmt::{Display, Formatter};
use std::ops::Deref;
use std::sync::Arc;

use rustdoc_types::ItemEnum;

use crate::language::callable_path::{CallPathGenericArgument, CallPathLifetime, CallPathType};
use crate::language::krate_name::dependency_name2package_id;
use crate::language::resolved_type::{Array, FunctionPointer, GenericArgument, Slice};
use crate::language::{CallPath, InvalidCallPath, RawPointer, Tuple, Type, TypeReference};
use crate::rustdoc::{
    CannotGetCrateData, CrateCollection, CrateCollectionExt, GlobalItemId, ResolvedItem,
};
use rustdoc_ext::RustdocKindExt;
use rustdoc_resolver::{GenericBindings, resolve_type};

use super::RawIdentifiers;
use super::fq_path::*;
use super::krate_name::CrateNameResolutionError;
use super::krate2package_id;
use super::resolved_type::GenericLifetimeParameter;

#[derive(Debug, Clone, Copy)]
pub enum PathKind {
    Callable,
    Type,
}

pub fn resolve_fq_path_type(
    path_type: &FQPathType,
    krate_collection: &CrateCollection,
) -> Result<Type, anyhow::Error> {
    match path_type {
        FQPathType::ResolvedPath(p) => {
            let resolved_item = find_rustdoc_item_type(&p.path, krate_collection)?.1;
            let item = &resolved_item.item;
            let used_by_package_id = resolved_item.item_id.package_id();
            let (global_type_id, base_type) = krate_collection
                .get_canonical_path_by_local_type_id(used_by_package_id, &item.id, None)?;
            let mut generic_arguments = vec![];
            let generic_param_def = match &item.inner {
                ItemEnum::Enum(e) => &e.generics.params,
                ItemEnum::Struct(s) => &s.generics.params,
                ItemEnum::Primitive(_) => &Vec::new(),
                other => {
                    anyhow::bail!(
                        "Generic parameters can either be set to structs or enums, \
                        but I found `{}`, {}.",
                        base_type.join("::"),
                        other.kind()
                    );
                }
            };

            let last_segment = &p
                .path
                .segments
                .last()
                .expect("Type with an empty path, impossible!");
            for (i, param_def) in generic_param_def.iter().enumerate() {
                let arg = if let Some(arg) = last_segment.generic_arguments.get(i) {
                    match arg {
                        FQGenericArgument::Type(t) => GenericArgument::TypeParameter(
                            resolve_fq_path_type(t, krate_collection)?,
                        ),
                        FQGenericArgument::Lifetime(l) => {
                            GenericArgument::Lifetime(l.clone().into())
                        }
                    }
                } else {
                    match &param_def.kind {
                        rustdoc_types::GenericParamDefKind::Lifetime { .. } => {
                            GenericArgument::Lifetime(GenericLifetimeParameter::from_name(
                                param_def.name.clone(),
                            ))
                        }
                        rustdoc_types::GenericParamDefKind::Type { default, .. } => {
                            let Some(default) = default else {
                                anyhow::bail!(
                                    "Every generic parameter must either be explicitly assigned or have a default. \
                                    `{}` in `{}` is unassigned and without a default.",
                                    param_def.name,
                                    base_type.join("::")
                                )
                            };
                            let ty = resolve_type(
                                default,
                                &resolved_item.item_id.package_id,
                                krate_collection,
                                &GenericBindings::default(),
                            )?;
                            GenericArgument::TypeParameter(ty)
                        }
                        rustdoc_types::GenericParamDefKind::Const { .. } => {
                            anyhow::bail!(
                                "Const generics are not supported yet. I can't process `{}` in `{}`",
                                param_def.name,
                                base_type.join("::")
                            )
                        }
                    }
                };
                generic_arguments.push(arg);
            }

            Ok(crate::language::resolved_type::PathType {
                package_id: global_type_id.package_id().to_owned(),
                rustdoc_id: Some(global_type_id.rustdoc_item_id),
                base_type: base_type.to_vec(),
                generic_arguments,
            }
            .into())
        }
        FQPathType::Reference(r) => Ok(Type::Reference(TypeReference {
            is_mutable: r.is_mutable,
            lifetime: r.lifetime.clone(),
            inner: Box::new(resolve_fq_path_type(&r.inner, krate_collection)?),
        })),
        FQPathType::Tuple(t) => {
            let elements = t
                .elements
                .iter()
                .map(|e| resolve_fq_path_type(e, krate_collection))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Type::Tuple(Tuple { elements }))
        }
        FQPathType::ScalarPrimitive(s) => Ok(Type::ScalarPrimitive(s.clone())),
        FQPathType::Slice(s) => {
            let inner = resolve_fq_path_type(&s.element, krate_collection)?;
            Ok(Type::Slice(Slice {
                element_type: Box::new(inner),
            }))
        }
        FQPathType::Array(a) => {
            let inner = resolve_fq_path_type(&a.element, krate_collection)?;
            Ok(Type::Array(Array {
                element_type: Box::new(inner),
                len: a.len,
            }))
        }
        FQPathType::RawPointer(r) => {
            let inner = resolve_fq_path_type(&r.inner, krate_collection)?;
            Ok(Type::RawPointer(RawPointer {
                is_mutable: r.is_mutable,
                inner: Box::new(inner),
            }))
        }
        FQPathType::FunctionPointer(fp) => {
            let inputs = fp
                .inputs
                .iter()
                .map(|e| resolve_fq_path_type(e, krate_collection))
                .collect::<Result<Vec<_>, _>>()?;
            let output = fp
                .output
                .as_ref()
                .map(|t| resolve_fq_path_type(t, krate_collection))
                .transpose()?;
            Ok(Type::FunctionPointer(FunctionPointer {
                inputs,
                output: output.map(Box::new),
            }))
        }
    }
}

/// Parses a fully qualified path from the given identifiers.
///
/// All paths must be fully qualified, meaning that they must start with a package name.
/// Using `crate::`/`super::`/`self::` is not allowed.
pub fn parse_fq_path(
    identifiers: &RawIdentifiers,
    graph: &guppy::graph::PackageGraph,
    kind: PathKind,
) -> Result<FQPath, ParseError> {
    let path = match kind {
        PathKind::Callable => CallPath::parse_callable_path(identifiers),
        PathKind::Type => CallPath::parse_type_path(identifiers),
    }?;
    parse_call_path(&path, identifiers, graph)
}

fn parse_call_path_generic_argument(
    arg: &CallPathGenericArgument,
    identifiers: &RawIdentifiers,
    graph: &guppy::graph::PackageGraph,
) -> Result<FQGenericArgument, ParseError> {
    match arg {
        CallPathGenericArgument::Type(t) => {
            parse_call_path_type(t, identifiers, graph).map(FQGenericArgument::Type)
        }
        CallPathGenericArgument::Lifetime(l) => match l {
            CallPathLifetime::Static => {
                Ok(FQGenericArgument::Lifetime(ResolvedPathLifetime::Static))
            }
            CallPathLifetime::Named(name) => Ok(FQGenericArgument::Lifetime(
                ResolvedPathLifetime::from_name(name.to_owned()),
            )),
        },
    }
}

fn parse_call_path_type(
    type_: &CallPathType,
    identifiers: &RawIdentifiers,
    graph: &guppy::graph::PackageGraph,
) -> Result<FQPathType, ParseError> {
    match type_ {
        CallPathType::ResolvedPath(p) => {
            let resolved_path = parse_call_path(p.path.deref(), identifiers, graph)?;
            Ok(FQPathType::ResolvedPath(FQResolvedPathType {
                path: Box::new(resolved_path),
            }))
        }
        CallPathType::Reference(r) => Ok(FQPathType::Reference(FQReference {
            is_mutable: r.is_mutable,
            lifetime: r.lifetime.clone(),
            inner: Box::new(parse_call_path_type(r.inner.deref(), identifiers, graph)?),
        })),
        CallPathType::Tuple(t) => {
            let mut elements = Vec::with_capacity(t.elements.len());
            for element in t.elements.iter() {
                elements.push(parse_call_path_type(element, identifiers, graph)?);
            }
            Ok(FQPathType::Tuple(FQTuple { elements }))
        }
        CallPathType::Slice(s) => {
            let element_type = parse_call_path_type(s.element_type.deref(), identifiers, graph)?;
            Ok(FQPathType::Slice(FQSlice {
                element: Box::new(element_type),
            }))
        }
    }
}

fn parse_call_path(
    path: &CallPath,
    identifiers: &RawIdentifiers,
    graph: &guppy::graph::PackageGraph,
) -> Result<FQPath, ParseError> {
    let mut segments = vec![];
    for raw_segment in &path.segments {
        let generic_arguments = raw_segment
            .generic_arguments
            .iter()
            .map(|arg| parse_call_path_generic_argument(arg, identifiers, graph))
            .collect::<Result<Vec<_>, _>>()?;
        let segment = FQPathSegment {
            ident: raw_segment.ident.to_string(),
            generic_arguments,
        };
        segments.push(segment);
    }

    let qself = if let Some(qself) = &path.qualified_self {
        Some(FQQualifiedSelf {
            position: qself.position,
            type_: parse_call_path_type(&qself.type_, identifiers, graph)?,
        })
    } else {
        None
    };

    let used_in = krate2package_id(
        &identifiers.created_at.package_name,
        &identifiers.created_at.package_version,
        graph,
    )
    .expect("Failed to resolve the created at coordinates to a package id");
    let package_id =
        dependency_name2package_id(&path.leading_path_segment().to_string(), &used_in, graph)
            .map_err(|source| PathMustBeAbsolute {
                relative_path: path.to_string(),
                source,
            })?;
    Ok(FQPath {
        segments,
        qualified_self: qself,
        package_id,
    })
}

/// Find the `rustdoc` items required to analyze the callable that `path` points to.
pub fn find_rustdoc_callable_items<'a>(
    path: &FQPath,
    krate_collection: &'a CrateCollection,
) -> Result<Result<CallableItem<'a>, UnknownPath>, CannotGetCrateData> {
    let krate = krate_collection.get_or_compute(&path.package_id)?;

    let path_without_generics: Vec<_> = path
        .segments
        .iter()
        .map(|path_segment| path_segment.ident.to_string())
        .collect();
    if let Ok(type_id) = krate.get_item_id_by_path(&path_without_generics, krate_collection)? {
        let i = krate_collection.get_item_by_global_type_id(&type_id);
        return Ok(Ok(CallableItem::Function(
            ResolvedItem {
                item: i,
                item_id: type_id,
            },
            path.clone(),
        )));
    }

    // The path might be pointing to a method, which is not a type.
    // We drop the last segment to see if we can get a hit on the struct/enum type
    // to which the method belongs.
    if path.segments.len() < 3 {
        // It has to be at least three segments—crate name, type name, method name.
        // If it's shorter than three, it's just an unknown path.
        return Ok(Err(UnknownPath(
            path.clone(),
            Arc::new(anyhow::anyhow!(
                "{} is too short to be a method path, but there is no function at that path",
                path
            )),
        )));
    }

    let qself = match path
        .qualified_self
        .as_ref()
        .map(|qself| {
            if let FQPathType::ResolvedPath(p) = &qself.type_ {
                find_rustdoc_item_type(&p.path, krate_collection)
                    .map_err(|e| UnknownPath(path.to_owned(), Arc::new(e.into())))
            } else {
                Err(UnknownPath(
                    path.clone(),
                    Arc::new(anyhow::anyhow!("Qualified self type is not a path")),
                ))
            }
        })
        .transpose()
    {
        Ok(x) => x.map(|(item, resolved_path)| (resolved_path, item)),
        Err(e) => return Ok(Err(e)),
    };

    let (method_name_segment, type_path_segments) = path.segments.split_last().unwrap();

    // Let's first try to see if the parent path points to a type, that we'll consider to be `Self`
    let method_owner_path = FQPath {
        segments: type_path_segments.to_vec(),
        qualified_self: None,
        package_id: path.package_id.clone(),
    };
    let (method_owner_path, method_owner_item) =
        match krate_collection.get_type_by_resolved_path(method_owner_path)? {
            Ok(p) => p,
            Err(e) => {
                return Ok(Err(UnknownPath(path.clone(), Arc::new(e.into()))));
            }
        };

    // If we're dealing with a trait method, we want to search in the docs of the trait itself
    // as well as the docs of the implementing type.
    let mut parent_items = match &qself {
        Some((item, _)) => vec![item, &method_owner_item],
        None => vec![&method_owner_item],
    };
    let method;
    let mut parent_item = parent_items.pop().unwrap();
    'outer: loop {
        let children_ids = match &parent_item.item.inner {
            ItemEnum::Struct(s) => &s.impls,
            ItemEnum::Enum(enum_) => &enum_.impls,
            ItemEnum::Trait(trait_) => &trait_.items,
            _ => {
                unreachable!()
            }
        };
        let search_krate = krate_collection.get_or_compute(&parent_item.item_id.package_id)?;
        for child_id in children_ids {
            let child = search_krate.get_item_by_local_type_id(child_id);
            match &child.inner {
                ItemEnum::Impl(impl_block) => {
                    // We are completely ignoring the bounds attached to the implementation block.
                    // This can lead to issues: the same method can be defined multiple
                    // times in different implementation blocks with non-overlapping constraints.
                    for impl_item_id in &impl_block.items {
                        let impl_item = search_krate.get_item_by_local_type_id(impl_item_id);
                        if impl_item.name.as_ref() == Some(&method_name_segment.ident)
                            && let ItemEnum::Function(_) = &impl_item.inner
                        {
                            method = Some(ResolvedItem {
                                item: impl_item,
                                item_id: GlobalItemId {
                                    package_id: search_krate.core.package_id.clone(),
                                    rustdoc_item_id: impl_item_id.to_owned(),
                                },
                            });
                            break 'outer;
                        }
                    }
                }
                ItemEnum::Function(_) => {
                    if child.name.as_ref() == Some(&method_name_segment.ident) {
                        method = Some(ResolvedItem {
                            item: child,
                            item_id: GlobalItemId {
                                package_id: search_krate.core.package_id.clone(),
                                rustdoc_item_id: child_id.to_owned(),
                            },
                        });
                        break 'outer;
                    }
                }
                i => {
                    dbg!(i);
                    unreachable!()
                }
            }
        }

        if let Some(next_parent) = parent_items.pop() {
            parent_item = next_parent;
        } else {
            method = None;
            break;
        }
    }

    let method_path = FQPath {
        segments: method_owner_path
            .segments
            .iter()
            .chain(std::iter::once(method_name_segment))
            .cloned()
            .collect(),
        qualified_self: path.qualified_self.clone(),
        package_id: parent_item.item_id.package_id.clone(),
    };
    if let Some(method) = method {
        Ok(Ok(CallableItem::Method {
            method_owner: (method_owner_item, method_owner_path),
            method: (method, method_path),
            qualified_self: qself,
        }))
    } else {
        Ok(Err(UnknownPath(
            path.clone(),
            Arc::new(anyhow::anyhow!(
                "There was no method named `{}` attached to `{}`",
                method_name_segment.ident,
                method_owner_path
            )),
        )))
    }
}

/// Find information about the type that this path points at.
/// It only works if the path points at a type (i.e. struct or enum).
/// It will return an error if the path points at a function or a method instead.
pub fn find_rustdoc_item_type<'a>(
    path: &FQPath,
    krate_collection: &'a CrateCollection,
) -> Result<(FQPath, ResolvedItem<'a>), UnknownPath> {
    krate_collection
        .get_type_by_resolved_path(path.clone())
        .map_err(|e| UnknownPath(path.to_owned(), Arc::new(e.into())))?
        .map_err(|e| UnknownPath(path.to_owned(), Arc::new(e.into())))
}

/// There are two key callables in Rust: functions and methods.
#[derive(Debug)]
pub enum CallableItem<'a> {
    /// Functions are free-standing and map to a single `rustdoc` item.
    Function(ResolvedItem<'a>, FQPath),
    /// Methods are associated with a type.
    /// They can either be inherent or trait methods.
    /// In the latter case, the `qualified_self` field will be populated with
    /// the `Self` type of the method.
    Method {
        /// The item to which the method belongs.
        /// This can be a trait, for a trait method, or a struct/enum for an inherent method.
        method_owner: (ResolvedItem<'a>, FQPath),
        method: (ResolvedItem<'a>, FQPath),
        /// The `self` type of the method.
        /// It's only populated when working with trait methods.
        qualified_self: Option<(ResolvedItem<'a>, FQPath)>,
    },
}

#[derive(Debug, thiserror::Error, Clone)]
pub enum ParseError {
    #[error(transparent)]
    InvalidPath(#[from] InvalidCallPath),
    #[error(transparent)]
    PathMustBeAbsolute(#[from] PathMustBeAbsolute),
}

#[derive(Debug, thiserror::Error, Clone)]
pub struct PathMustBeAbsolute {
    pub(crate) relative_path: String,
    #[source]
    pub(crate) source: CrateNameResolutionError,
}

impl Display for PathMustBeAbsolute {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "`{}` is not a fully-qualified import path.",
            self.relative_path
        )
    }
}

#[derive(thiserror::Error, Debug, Clone)]
pub struct UnknownPath(pub FQPath, #[source] Arc<anyhow::Error>);

impl Display for UnknownPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let path = &self.0;
        let krate = path.crate_name().to_string();
        write!(
            f,
            "I could not find '{path}' in the auto-generated documentation for '{krate}'."
        )
    }
}
