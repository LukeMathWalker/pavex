use std::fmt::Write;
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use anyhow::Context;
use bimap::BiHashMap;
use guppy::PackageId;
use indexmap::IndexSet;
use itertools::Itertools;
use quote::format_ident;

use pavex_bp_schema::{CreatedAt, RawIdentifiers};
use rustdoc_types::ItemEnum;

use crate::compiler::resolvers::{GenericBindings, resolve_type};
use crate::language::callable_path::{
    CallPathGenericArgument, CallPathLifetime, CallPathSegment, CallPathType,
};
use crate::language::krate_name::dependency_name2package_id;
use crate::language::resolved_type::{GenericArgument, Lifetime, ScalarPrimitive, Slice};
use crate::language::{CallPath, InvalidCallPath, ResolvedType, Tuple, TypeReference};
use crate::rustdoc::{
    CORE_PACKAGE_ID, CannotGetCrateData, CrateCollection, GlobalItemId, ResolvedItem,
    RustdocKindExt,
};

use super::krate_name::CrateNameResolutionError;
use super::resolved_type::GenericLifetimeParameter;

/// A resolved import path.
///
/// What does "resolved" mean in this contest?
///
/// `ResolvedPath` ensures that all paths are "fully qualified"—i.e.
/// the first path segment is either the name of the current package or the name of a
/// crate listed as a dependency of the current package.
///
/// It also performs basic normalization.
/// There are ways, in Rust, to have different paths pointing at the same type.
///
/// E.g. `crate_name::TypeName` and `::crate_name::TypeName` are equivalent when `crate_name`
/// is a third-party dependency of your project. `ResolvedPath` reduces those two different
/// representations to a canonical one, allowing for deduplication.
///
/// Another common scenario: dependency renaming.
/// `crate_name::TypeName` and `renamed_crate_name::TypeName` can be equivalent if `crate_name`
/// has been renamed to `renamed_crate_name` in the `Cargo.toml` of the package that declares/uses
/// the path. `ResolvedPath` takes this into account by using the `PackageId` of the target
/// crate as the authoritative answer to "What crate does this path belong to?". This is unique
/// and well-defined within a `cargo` workspace.
#[derive(Clone, Debug, Eq)]
pub struct ResolvedPath {
    pub segments: Vec<ResolvedPathSegment>,
    /// The qualified self of the path, if any.
    ///
    /// E.g. `Type` in `<Type as Trait>::Method`.
    pub qualified_self: Option<ResolvedPathQualifiedSelf>,
    /// The package id of the crate that this path belongs to.
    pub package_id: PackageId,
}

impl ResolvedPath {
    /// Collect all the package ids that are referenced in this path.
    ///
    /// This includes the package id of the crate that this path belongs to
    /// as well as the package ids of all the crates that are referenced in its generic
    /// arguments and qualified self (if any).
    pub(crate) fn collect_package_ids<'a>(&'a self, package_ids: &mut IndexSet<&'a PackageId>) {
        package_ids.insert(&self.package_id);
        for segment in &self.segments {
            for generic_argument in &segment.generic_arguments {
                match generic_argument {
                    ResolvedPathGenericArgument::Type(t) => {
                        t.collect_package_ids(package_ids);
                    }
                    ResolvedPathGenericArgument::Lifetime(_) => {}
                }
            }
        }
        if let Some(qself) = &self.qualified_self {
            qself.type_.collect_package_ids(package_ids);
        }
    }
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct ResolvedPathQualifiedSelf {
    pub position: usize,
    pub type_: ResolvedPathType,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct ResolvedPathSegment {
    pub ident: String,
    pub generic_arguments: Vec<ResolvedPathGenericArgument>,
}

impl ResolvedPathSegment {
    /// Create a new segment without generic arguments.
    pub fn new(ident: String) -> Self {
        Self {
            ident,
            generic_arguments: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum ResolvedPathGenericArgument {
    Type(ResolvedPathType),
    Lifetime(ResolvedPathLifetime),
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum ResolvedPathLifetime {
    Static,
    Named(String),
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum ResolvedPathType {
    ResolvedPath(ResolvedPathResolvedPathType),
    Reference(ResolvedPathReference),
    Tuple(ResolvedPathTuple),
    ScalarPrimitive(ScalarPrimitive),
    Slice(ResolvedPathSlice),
}

impl ResolvedPathType {
    pub fn resolve(
        &self,
        krate_collection: &CrateCollection,
    ) -> Result<ResolvedType, anyhow::Error> {
        match self {
            ResolvedPathType::ResolvedPath(p) => {
                let resolved_item = p.path.find_rustdoc_item_type(krate_collection)?.1;
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
                            ResolvedPathGenericArgument::Type(t) => {
                                GenericArgument::TypeParameter(t.resolve(krate_collection)?)
                            }
                            ResolvedPathGenericArgument::Lifetime(l) => match l {
                                ResolvedPathLifetime::Static => {
                                    GenericArgument::Lifetime(GenericLifetimeParameter::Static)
                                }
                                ResolvedPathLifetime::Named(name) => GenericArgument::Lifetime(
                                    GenericLifetimeParameter::Named(name.clone()),
                                ),
                            },
                        }
                    } else {
                        match &param_def.kind {
                            rustdoc_types::GenericParamDefKind::Lifetime { .. } => {
                                GenericArgument::Lifetime(GenericLifetimeParameter::Named(
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
            ResolvedPathType::Reference(r) => Ok(ResolvedType::Reference(TypeReference {
                is_mutable: r.is_mutable,
                lifetime: r.lifetime.clone(),
                inner: Box::new(r.inner.resolve(krate_collection)?),
            })),
            ResolvedPathType::Tuple(t) => {
                let elements = t
                    .elements
                    .iter()
                    .map(|e| e.resolve(krate_collection))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(ResolvedType::Tuple(Tuple { elements }))
            }
            ResolvedPathType::ScalarPrimitive(s) => Ok(ResolvedType::ScalarPrimitive(s.clone())),
            ResolvedPathType::Slice(s) => {
                let inner = s.element.resolve(krate_collection)?;
                Ok(ResolvedType::Slice(Slice {
                    element_type: Box::new(inner),
                }))
            }
        }
    }

    /// Collect all the package ids that are referenced in this path type.
    ///
    /// This includes the package id of the crate that this path belongs to
    /// as well as the package ids of all the crates that are referenced in its generic
    /// arguments (if any).
    fn collect_package_ids<'a>(&'a self, package_ids: &mut IndexSet<&'a PackageId>) {
        match self {
            ResolvedPathType::ResolvedPath(p) => {
                p.path.collect_package_ids(package_ids);
            }
            ResolvedPathType::Reference(r) => {
                r.inner.collect_package_ids(package_ids);
            }
            ResolvedPathType::Tuple(t) => {
                for element in &t.elements {
                    element.collect_package_ids(package_ids);
                }
            }
            ResolvedPathType::ScalarPrimitive(_) => {
                package_ids.insert(&CORE_PACKAGE_ID);
            }
            ResolvedPathType::Slice(s) => {
                s.element.collect_package_ids(package_ids);
            }
        }
    }
}

impl From<ResolvedType> for ResolvedPathType {
    fn from(value: ResolvedType) -> Self {
        match value {
            ResolvedType::ResolvedPath(p) => {
                let mut segments: Vec<ResolvedPathSegment> = p
                    .base_type
                    .iter()
                    .map(|s| ResolvedPathSegment {
                        ident: s.to_string(),
                        generic_arguments: vec![],
                    })
                    .collect();
                if let Some(segment) = segments.last_mut() {
                    segment.generic_arguments = p
                        .generic_arguments
                        .into_iter()
                        .map(|t| match t {
                            GenericArgument::TypeParameter(t) => {
                                ResolvedPathGenericArgument::Type(t.into())
                            }
                            GenericArgument::Lifetime(l) => match l {
                                GenericLifetimeParameter::Static => {
                                    ResolvedPathGenericArgument::Lifetime(
                                        ResolvedPathLifetime::Static,
                                    )
                                }
                                GenericLifetimeParameter::Named(name) => {
                                    ResolvedPathGenericArgument::Lifetime(
                                        ResolvedPathLifetime::Named(name),
                                    )
                                }
                            },
                        })
                        .collect();
                }
                ResolvedPathType::ResolvedPath(ResolvedPathResolvedPathType {
                    path: Box::new(ResolvedPath {
                        segments,
                        qualified_self: None,
                        package_id: p.package_id,
                    }),
                })
            }
            ResolvedType::Reference(r) => ResolvedPathType::Reference(ResolvedPathReference {
                is_mutable: r.is_mutable,
                lifetime: r.lifetime,
                inner: Box::new((*r.inner).into()),
            }),
            ResolvedType::Tuple(t) => ResolvedPathType::Tuple(ResolvedPathTuple {
                elements: t.elements.into_iter().map(|e| e.into()).collect(),
            }),
            ResolvedType::ScalarPrimitive(s) => ResolvedPathType::ScalarPrimitive(s),
            ResolvedType::Slice(s) => ResolvedPathType::Slice(ResolvedPathSlice {
                element: Box::new((*s.element_type).into()),
            }),
            ResolvedType::Generic(_) => {
                // ResolvedPath doesn't support unassigned generic parameters.
                unreachable!("UnassignedGeneric")
            }
        }
    }
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct ResolvedPathResolvedPathType {
    pub path: Box<ResolvedPath>,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct ResolvedPathReference {
    pub is_mutable: bool,
    pub lifetime: Lifetime,
    pub inner: Box<ResolvedPathType>,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct ResolvedPathTuple {
    pub elements: Vec<ResolvedPathType>,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct ResolvedPathSlice {
    pub element: Box<ResolvedPathType>,
}

impl PartialEq for ResolvedPath {
    fn eq(&self, other: &Self) -> bool {
        // Using destructuring syntax to make sure we get a compiler error
        // if a new field gets added, as a reminder to update this Hash implementation.
        let Self {
            segments,
            qualified_self,
            package_id,
        } = self;
        let Self {
            segments: other_segments,
            qualified_self: other_qualified_self,
            package_id: other_package_id,
        } = other;
        let is_equal = package_id == other_package_id
            && segments.len() == other_segments.len()
            && qualified_self == other_qualified_self;
        if is_equal {
            // We want to ignore the first segment of the path, because dependencies can be
            // renamed and this can lead to equivalent paths not being considered equal.
            // Given that we already have the package id as part of the type, it is safe
            // to disregard the first segment when determining equality.
            let self_segments = segments.iter().skip(1);
            let other_segments = other_segments.iter().skip(1);
            for (self_segment, other_segment) in self_segments.zip_eq(other_segments) {
                if self_segment != other_segment {
                    return false;
                }
            }
            true
        } else {
            false
        }
    }
}

impl Hash for ResolvedPath {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Using destructuring syntax to make sure we get a compiler error
        // if a new field gets added, as a reminder to update this Hash implementation.
        let Self {
            segments,
            qualified_self,
            package_id,
        } = self;
        package_id.hash(state);
        qualified_self.hash(state);
        // We want to ignore the first segment of the path, because dependencies can be
        // renamed and this can lead to equivalent paths not being considered equal.
        // Given that we already have the package id as part of the type, it is safe
        // to disregard the first segment when determining equality.
        let self_segments = segments.iter().skip(1);
        for segment in self_segments {
            segment.hash(state)
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum PathKind {
    Callable,
    Type,
}

impl ResolvedPath {
    pub fn parse(
        identifiers: &RawIdentifiers,
        graph: &guppy::graph::PackageGraph,
        kind: PathKind,
    ) -> Result<Self, ParseError> {
        fn replace_crate_in_path_with_registration_crate(p: &mut CallPath, created_at: &CreatedAt) {
            if p.leading_path_segment() == "crate" {
                let first_segment = p
                    .segments
                    .first_mut()
                    .expect("Bug: a `CallPath` with no path segments!");
                // Unwrapping here is safe: there is always at least one path segment in a successfully
                // parsed `ExprPath`.
                // We must make sure to normalize the crate name, since it may contain hyphens.
                first_segment.ident = format_ident!("{}", created_at.package_name.replace('-', "_"));
            } else if p.leading_path_segment() == "self" {
                // We make the path absolute by adding the module path to its beginning.
                let old_segments = std::mem::take(&mut p.segments);
                let new_segments: Vec<_> = created_at
                    .module_path
                    .split("::")
                    .map(|s| {
                        let ident = format_ident!("{}", s.trim().to_owned());
                        CallPathSegment {
                            ident,
                            generic_arguments: vec![],
                        }
                    })
                    .chain(old_segments.into_iter().skip(1))
                    .collect();
                p.segments = new_segments;
            } else if p.leading_path_segment() == "super" {
                // We make the path absolute by adding replacing `super` with the relevant
                // parts of the module path.
                let n_super: usize = {
                    let mut n_super = 0;
                    let iter = p.segments.iter();
                    for p in iter {
                        if p.ident == "super" {
                            n_super += 1;
                        } else {
                            break;
                        }
                    }
                    n_super
                };
                let old_segments = std::mem::take(&mut p.segments);
                // The path is relative to the current module.
                // We "rebase" it to get an absolute path.
                let module_segments: Vec<_> = created_at
                    .module_path
                    .split("::")
                    .map(|s| {
                        let ident = format_ident!("{}", s.trim().to_owned());
                        CallPathSegment {
                            ident,
                            generic_arguments: vec![],
                        }
                    })
                    .collect();
                let n_module_segments = module_segments.len();
                let new_segments: Vec<_> = module_segments
                    .into_iter()
                    .take(n_module_segments - n_super)
                    .chain(old_segments.into_iter().skip(n_super))
                    .collect();
                p.segments = new_segments;
            }
            for segment in p.segments.iter_mut() {
                for generic_argument in segment.generic_arguments.iter_mut() {
                    match generic_argument {
                        CallPathGenericArgument::Type(t) => {
                            replace_crate_in_type_with_registration_crate(t, created_at);
                        }
                        CallPathGenericArgument::Lifetime(_) => {}
                    }
                }
            }
        }

        fn replace_crate_in_type_with_registration_crate(
            t: &mut CallPathType,
            created_at: &CreatedAt,
        ) {
            match t {
                CallPathType::ResolvedPath(p) => {
                    replace_crate_in_path_with_registration_crate(p.path.deref_mut(), created_at)
                }
                CallPathType::Reference(r) => {
                    replace_crate_in_type_with_registration_crate(r.inner.deref_mut(), created_at)
                }
                CallPathType::Tuple(t) => {
                    for element in t.elements.iter_mut() {
                        replace_crate_in_type_with_registration_crate(element, created_at);
                    }
                }
                CallPathType::Slice(s) => {
                    replace_crate_in_type_with_registration_crate(
                        s.element_type.deref_mut(),
                        created_at,
                    );
                }
            }
        }

        let mut path = match kind {
            PathKind::Callable => CallPath::parse_callable_path(identifiers),
            PathKind::Type => CallPath::parse_type_path(identifiers),
        }?;
        replace_crate_in_path_with_registration_crate(&mut path, identifiers.created_at());
        if let Some(qself) = &mut path.qualified_self {
            replace_crate_in_type_with_registration_crate(
                &mut qself.type_,
                identifiers.created_at(),
            );
        }
        Self::parse_call_path(&path, identifiers, graph)
    }

    fn parse_call_path_generic_argument(
        arg: &CallPathGenericArgument,
        identifiers: &RawIdentifiers,
        graph: &guppy::graph::PackageGraph,
    ) -> Result<ResolvedPathGenericArgument, ParseError> {
        match arg {
            CallPathGenericArgument::Type(t) => Self::parse_call_path_type(t, identifiers, graph)
                .map(ResolvedPathGenericArgument::Type),
            CallPathGenericArgument::Lifetime(l) => match l {
                CallPathLifetime::Static => Ok(ResolvedPathGenericArgument::Lifetime(
                    ResolvedPathLifetime::Static,
                )),
                CallPathLifetime::Named(name) => Ok(ResolvedPathGenericArgument::Lifetime(
                    ResolvedPathLifetime::Named(name.to_owned()),
                )),
            },
        }
    }

    fn parse_call_path_type(
        type_: &CallPathType,
        identifiers: &RawIdentifiers,
        graph: &guppy::graph::PackageGraph,
    ) -> Result<ResolvedPathType, ParseError> {
        match type_ {
            CallPathType::ResolvedPath(p) => {
                let resolved_path = Self::parse_call_path(p.path.deref(), identifiers, graph)?;
                Ok(ResolvedPathType::ResolvedPath(
                    ResolvedPathResolvedPathType {
                        path: Box::new(resolved_path),
                    },
                ))
            }
            CallPathType::Reference(r) => Ok(ResolvedPathType::Reference(ResolvedPathReference {
                is_mutable: r.is_mutable,
                lifetime: r.lifetime.clone(),
                inner: Box::new(Self::parse_call_path_type(
                    r.inner.deref(),
                    identifiers,
                    graph,
                )?),
            })),
            CallPathType::Tuple(t) => {
                let mut elements = Vec::with_capacity(t.elements.len());
                for element in t.elements.iter() {
                    elements.push(Self::parse_call_path_type(element, identifiers, graph)?);
                }
                Ok(ResolvedPathType::Tuple(ResolvedPathTuple { elements }))
            }
            CallPathType::Slice(s) => {
                let element_type =
                    Self::parse_call_path_type(s.element_type.deref(), identifiers, graph)?;
                Ok(ResolvedPathType::Slice(ResolvedPathSlice {
                    element: Box::new(element_type),
                }))
            }
        }
    }

    fn parse_call_path(
        path: &CallPath,
        identifiers: &RawIdentifiers,
        graph: &guppy::graph::PackageGraph,
    ) -> Result<Self, ParseError> {
        let mut segments = vec![];
        for raw_segment in &path.segments {
            let generic_arguments = raw_segment
                .generic_arguments
                .iter()
                .map(|arg| Self::parse_call_path_generic_argument(arg, identifiers, graph))
                .collect::<Result<Vec<_>, _>>()?;
            let segment = ResolvedPathSegment {
                ident: raw_segment.ident.to_string(),
                generic_arguments,
            };
            segments.push(segment);
        }

        let qself = if let Some(qself) = &path.qualified_self {
            Some(ResolvedPathQualifiedSelf {
                position: qself.position,
                type_: Self::parse_call_path_type(&qself.type_, identifiers, graph)?,
            })
        } else {
            None
        };

        let package_id = dependency_name2package_id(
            &path.leading_path_segment().to_string(),
            &identifiers.created_at().package_name,
            graph,
        )
        .map_err(|source| PathMustBeAbsolute {
            relative_path: path.to_string(),
            source,
        })?;
        Ok(Self {
            segments,
            qualified_self: qself,
            package_id,
        })
    }

    /// Return the name of the crate that this type belongs to.
    ///
    /// E.g. `my_crate::my_module::MyType` will return `my_crate`.
    pub fn crate_name(&self) -> &str {
        // This unwrap never fails thanks to the validation done in `parse`
        &self.segments.first().unwrap().ident
    }

    /// Find the `rustdoc` items requied to analyze the callable that `self` points to.
    pub fn find_rustdoc_callable_items<'a>(
        &self,
        krate_collection: &'a CrateCollection,
    ) -> Result<Result<CallableItem<'a>, UnknownPath>, CannotGetCrateData> {
        let krate = krate_collection.get_or_compute_crate_by_package_id(&self.package_id)?;

        let path_without_generics: Vec<_> = self
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
                self.clone(),
            )));
        }

        // The path might be pointing to a method, which is not a type.
        // We drop the last segment to see if we can get a hit on the struct/enum type
        // to which the method belongs.
        if self.segments.len() < 3 {
            // It has to be at least three segments—crate name, type name, method name.
            // If it's shorter than three, it's just an unknown path.
            return Ok(Err(UnknownPath(
                self.clone(),
                Arc::new(anyhow::anyhow!(
                    "{} is too short to be a method path, but there is no function at that path",
                    self
                )),
            )));
        }

        let qself = match self
            .qualified_self
            .as_ref()
            .map(|qself| {
                if let ResolvedPathType::ResolvedPath(p) = &qself.type_ {
                    p.path
                        .find_rustdoc_item_type(krate_collection)
                        .map_err(|e| UnknownPath(self.to_owned(), Arc::new(e.into())))
                } else {
                    Err(UnknownPath(
                        self.clone(),
                        Arc::new(anyhow::anyhow!("Qualified self type is not a path")),
                    ))
                }
            })
            .transpose()
        {
            Ok(x) => x.map(|(item, path)| (path, item)),
            Err(e) => return Ok(Err(e)),
        };

        let (method_name_segment, type_path_segments) = self.segments.split_last().unwrap();

        // Let's first try to see if the parent path points to a type, that we'll consider to be `Self`
        let method_owner_path = ResolvedPath {
            segments: type_path_segments.to_vec(),
            qualified_self: None,
            package_id: self.package_id.clone(),
        };
        let (method_owner_path, method_owner_item) =
            match krate_collection.get_type_by_resolved_path(method_owner_path)? {
                Ok(p) => p,
                Err(e) => {
                    return Ok(Err(UnknownPath(self.clone(), Arc::new(e.into()))));
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
            let search_krate = krate_collection
                .get_or_compute_crate_by_package_id(&parent_item.item_id.package_id)?;
            for child_id in children_ids {
                let child = search_krate.get_item_by_local_type_id(child_id);
                match &child.inner {
                    ItemEnum::Impl(impl_block) => {
                        // We are completely ignoring the bounds attached to the implementation block.
                        // This can lead to issues: the same method can be defined multiple
                        // times in different implementation blocks with non-overlapping constraints.
                        for impl_item_id in &impl_block.items {
                            let impl_item = search_krate.get_item_by_local_type_id(impl_item_id);
                            if impl_item.name.as_ref() == Some(&method_name_segment.ident) {
                                if let ItemEnum::Function(_) = &impl_item.inner {
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

        let method_path = ResolvedPath {
            segments: method_owner_path
                .segments
                .iter()
                .chain(std::iter::once(method_name_segment))
                .cloned()
                .collect(),
            qualified_self: self.qualified_self.clone(),
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
                self.clone(),
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
        &self,
        krate_collection: &'a CrateCollection,
    ) -> Result<(ResolvedPath, ResolvedItem<'a>), UnknownPath> {
        krate_collection
            .get_type_by_resolved_path(self.clone())
            .map_err(|e| UnknownPath(self.to_owned(), Arc::new(e.into())))?
            .map_err(|e| UnknownPath(self.to_owned(), Arc::new(e.into())))
    }

    pub fn render_path(&self, id2name: &BiHashMap<PackageId, String>, buffer: &mut String) {
        let crate_name = id2name
            .get_by_left(&self.package_id)
            .with_context(|| {
                format!(
                    "The package id '{}' is missing from the id<>name mapping for crates.",
                    self.package_id
                )
            })
            .unwrap();
        let mut qself_closing_wedge_index = None;
        if let Some(qself) = &self.qualified_self {
            write!(buffer, "<").unwrap();
            qself.type_.render_path(id2name, buffer);
            write!(buffer, " as ",).unwrap();
            qself_closing_wedge_index = Some(qself.position.saturating_sub(1));
        }
        write!(buffer, "{crate_name}").unwrap();
        for (index, path_segment) in self.segments[1..].iter().enumerate() {
            write!(buffer, "::{}", path_segment.ident).unwrap();
            let generic_arguments = &path_segment.generic_arguments;
            if !generic_arguments.is_empty() {
                write!(buffer, "::<").unwrap();
                let mut arguments = generic_arguments.iter().peekable();
                while let Some(argument) = arguments.next() {
                    argument.render_path(id2name, buffer);
                    if arguments.peek().is_some() {
                        write!(buffer, ", ").unwrap();
                    }
                }
                write!(buffer, ">").unwrap();
            }
            if Some(index + 1) == qself_closing_wedge_index {
                write!(buffer, ">").unwrap();
            }
        }
    }
}

/// There are two key callables in Rust: functions and methods.
#[derive(Debug)]
pub enum CallableItem<'a> {
    /// Functions are free-standing and map to a single `rustdoc` item.
    Function(ResolvedItem<'a>, ResolvedPath),
    /// Methods are associated with a type.
    /// They can either be inherent or trait methods.
    /// In the latter case, the `qualified_self` field will be populated with
    /// the `Self` type of the method.
    Method {
        /// The item to which the method belongs.
        /// This can be a trait, for a trait method, or a struct/enum for an inherent method.
        method_owner: (ResolvedItem<'a>, ResolvedPath),
        method: (ResolvedItem<'a>, ResolvedPath),
        /// The `self` type of the method.
        /// It's only populated when working with trait methods.
        qualified_self: Option<(ResolvedItem<'a>, ResolvedPath)>,
    },
}

impl ResolvedPathType {
    pub fn render_path(&self, id2name: &BiHashMap<PackageId, String>, buffer: &mut String) {
        match self {
            ResolvedPathType::ResolvedPath(p) => p.render_path(id2name, buffer),
            ResolvedPathType::Reference(r) => r.render_path(id2name, buffer),
            ResolvedPathType::Tuple(t) => t.render_path(id2name, buffer),
            ResolvedPathType::ScalarPrimitive(s) => {
                write!(buffer, "{s}").unwrap();
            }
            ResolvedPathType::Slice(s) => s.render_path(id2name, buffer),
        }
    }
}

impl ResolvedPathSlice {
    pub fn render_path(&self, id2name: &BiHashMap<PackageId, String>, buffer: &mut String) {
        write!(buffer, "[").unwrap();
        self.element.render_path(id2name, buffer);
        write!(buffer, "]").unwrap();
    }
}

impl ResolvedPathGenericArgument {
    pub fn render_path(&self, id2name: &BiHashMap<PackageId, String>, buffer: &mut String) {
        match self {
            ResolvedPathGenericArgument::Type(t) => {
                t.render_path(id2name, buffer);
            }
            ResolvedPathGenericArgument::Lifetime(l) => match l {
                ResolvedPathLifetime::Static => {
                    write!(buffer, "'static").unwrap();
                }
                ResolvedPathLifetime::Named(_) => {
                    // TODO: we should have proper name mapping for lifetimes here, but we know all our current
                    //   usecases will work with lifetime elision.
                    write!(buffer, "'_").unwrap();
                }
            },
        }
    }
}

impl ResolvedPathResolvedPathType {
    pub fn render_path(&self, id2name: &BiHashMap<PackageId, String>, buffer: &mut String) {
        self.path.render_path(id2name, buffer);
    }
}

impl ResolvedPathTuple {
    pub fn render_path(&self, id2name: &BiHashMap<PackageId, String>, buffer: &mut String) {
        write!(buffer, "(").unwrap();
        let mut types = self.elements.iter().peekable();
        while let Some(ty) = types.next() {
            ty.render_path(id2name, buffer);
            if types.peek().is_some() {
                write!(buffer, ", ").unwrap();
            }
        }
        write!(buffer, ")").unwrap();
    }
}

impl ResolvedPathReference {
    pub fn render_path(&self, id2name: &BiHashMap<PackageId, String>, buffer: &mut String) {
        write!(buffer, "&").unwrap();
        if self.is_mutable {
            write!(buffer, "mut ").unwrap();
        }
        self.inner.render_path(id2name, buffer);
    }
}

impl Display for ResolvedPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let last_segment_index = self.segments.len().saturating_sub(1);
        let mut qself_closing_wedge_index = None;
        if let Some(qself) = &self.qualified_self {
            write!(f, "<{} as ", qself.type_)?;
            qself_closing_wedge_index = Some(qself.position.saturating_sub(1))
        }
        for (i, segment) in self.segments.iter().enumerate() {
            write!(f, "{segment}")?;
            if Some(i) == qself_closing_wedge_index {
                write!(f, ">")?;
            }
            if i != last_segment_index {
                write!(f, "::")?;
            }
        }
        Ok(())
    }
}

impl Display for ResolvedPathType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ResolvedPathType::ResolvedPath(p) => write!(f, "{}", p),
            ResolvedPathType::Reference(r) => write!(f, "{}", r),
            ResolvedPathType::Tuple(t) => write!(f, "{}", t),
            ResolvedPathType::ScalarPrimitive(s) => {
                write!(f, "{}", s)
            }
            ResolvedPathType::Slice(s) => {
                write!(f, "{}", s)
            }
        }
    }
}

impl Display for ResolvedPathSlice {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}]", self.element)
    }
}

impl Display for ResolvedPathResolvedPathType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.path)
    }
}

impl Display for ResolvedPathReference {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "&")?;
        if self.is_mutable {
            write!(f, "mut ")?;
        }
        write!(f, "{}", self.inner)
    }
}

impl Display for ResolvedPathTuple {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "(")?;
        let last_element_index = self.elements.len().saturating_sub(1);
        for (i, element) in self.elements.iter().enumerate() {
            write!(f, "{}", element)?;
            if i != last_element_index {
                write!(f, ", ")?;
            }
        }
        write!(f, ")")
    }
}

impl Display for ResolvedPathSegment {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.ident)?;
        if !self.generic_arguments.is_empty() {
            write!(f, "::<")?;
        }
        let last_argument_index = self.generic_arguments.len().saturating_sub(1);
        for (j, argument) in self.generic_arguments.iter().enumerate() {
            write!(f, "{argument}")?;
            if j != last_argument_index {
                write!(f, ", ")?;
            }
        }
        if !self.generic_arguments.is_empty() {
            write!(f, ">")?;
        }
        Ok(())
    }
}

impl Display for ResolvedPathGenericArgument {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ResolvedPathGenericArgument::Type(t) => {
                write!(f, "{}", t)
            }
            ResolvedPathGenericArgument::Lifetime(l) => {
                write!(f, "{}", l)
            }
        }
    }
}

impl Display for ResolvedPathLifetime {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ResolvedPathLifetime::Static => write!(f, "'static"),
            ResolvedPathLifetime::Named(name) => write!(f, "{}", name),
        }
    }
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
pub struct UnknownPath(pub ResolvedPath, #[source] Arc<anyhow::Error>);

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
