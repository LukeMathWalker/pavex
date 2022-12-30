use std::fmt::Write;
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::Deref;

use anyhow::Context;
use bimap::BiHashMap;
use guppy::PackageId;
use itertools::Itertools;
use quote::format_ident;

use pavex_builder::RawCallableIdentifiers;

use crate::language::{CallPath, InvalidCallPath};
use crate::rustdoc::{CrateCollection, GlobalItemId};
use crate::rustdoc::{ResolvedItem, TOOLCHAIN_CRATES};

/// A resolved import path.
///
/// What does "resolved" mean in this contest?
///
/// `ResolvedPath` ensures that all paths are "fully qualified" - i.e.
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
    pub qualified_self: Option<ResolvedPathQualifiedSelf>,
    pub package_id: PackageId,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct ResolvedPathQualifiedSelf {
    pub position: usize,
    pub path: Box<ResolvedPath>,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct ResolvedPathSegment {
    pub ident: String,
    pub generic_arguments: Vec<ResolvedPath>,
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
            && segments.len() == segments.len()
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

impl ResolvedPath {
    pub fn parse(
        identifiers: &RawCallableIdentifiers,
        graph: &guppy::graph::PackageGraph,
    ) -> Result<Self, ParseError> {
        fn replace_crate_with_registration_crate(
            p: &mut CallPath,
            identifiers: &RawCallableIdentifiers,
        ) {
            if p.leading_path_segment() == "crate" {
                let first_segment = p
                    .segments
                    .first_mut()
                    .expect("Bug: a `CallPath` with no path segments!");
                // Unwrapping here is safe: there is always at least one path segment in a successfully
                // parsed `ExprPath`.
                first_segment.ident = format_ident!("{}", identifiers.registered_at());
            }
            for segment in p.segments.iter_mut() {
                for generic_argument in segment.generic_arguments.iter_mut() {
                    replace_crate_with_registration_crate(generic_argument, identifiers);
                }
            }
        }

        let mut path = CallPath::parse(identifiers)?;
        replace_crate_with_registration_crate(&mut path, identifiers);
        if let Some(qself) = &mut path.qualified_self {
            replace_crate_with_registration_crate(&mut qself.path, identifiers);
        }
        Self::parse_call_path(&path, identifiers, graph)
    }

    fn parse_call_path(
        path: &CallPath,
        identifiers: &RawCallableIdentifiers,
        graph: &guppy::graph::PackageGraph,
    ) -> Result<Self, ParseError> {
        let registered_at = identifiers.registered_at();
        let krate_name_candidate = path.leading_path_segment().to_string();

        let mut segments = vec![];
        for raw_segment in &path.segments {
            let generic_arguments = raw_segment
                .generic_arguments
                .iter()
                .map(|arg| Self::parse_call_path(arg, identifiers, graph))
                .collect::<Result<Vec<_>, _>>()?;
            let segment = ResolvedPathSegment {
                ident: raw_segment.ident.to_string(),
                generic_arguments,
            };
            segments.push(segment);
        }

        let qself = match &path.qualified_self {
            Some(qself) => {
                let qself_path = Self::parse_call_path(&qself.path, identifiers, graph)?;
                Some(ResolvedPathQualifiedSelf {
                    position: qself.position,
                    path: Box::new(qself_path),
                })
            }
            None => None,
        };

        let registration_package = graph.packages()
            .find(|p| p.name() == registered_at)
            .expect("There is no package in the current workspace whose name matches the registration crate for these identifiers");
        let package_id = if registration_package.name() == krate_name_candidate {
            registration_package.id().to_owned()
        } else if let Some(dependency) = registration_package
            .direct_links()
            .find(|d| d.resolved_name() == krate_name_candidate)
        {
            dependency.to().id().to_owned()
        } else if TOOLCHAIN_CRATES.contains(&krate_name_candidate.as_str()) {
            PackageId::new(krate_name_candidate.clone())
        } else {
            return Err(PathMustBeAbsolute {
                raw_identifiers: identifiers.to_owned(),
                relative_path: path.to_string(),
            }
            .into());
        };
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

    /// Return the unequivocal [`GlobalItemId`] that this path points at.
    ///
    /// This method only works for structs, enums and free functions.
    /// It won't work for methods!
    pub fn find_type_id(
        &self,
        krate_collection: &CrateCollection,
    ) -> Result<GlobalItemId, UnknownPath> {
        // TODO: remove unwrap here
        let krate = krate_collection
            .get_or_compute_crate_by_package_id(&self.package_id)
            .unwrap();
        let path_segments: Vec<_> = self
            .segments
            .iter()
            .map(|path_segment| path_segment.ident.to_string())
            .collect();
        match krate.get_type_id_by_path(&path_segments) {
            Ok(type_id) => Ok(type_id.to_owned()),
            Err(e) => Err(UnknownPath(self.to_owned(), e.into())),
        }
    }

    /// Find information about the type that this path points at.
    /// It also returns the type of the qualified self, if it is present.
    ///
    /// E.g. `MyType` will return `(MyType, None)`.
    /// `<MyType as MyTrait>::trait_method` will return `(MyType, Some(MyTrait::trait_method))`.
    pub fn find_rustdoc_items<'a>(
        &self,
        krate_collection: &'a CrateCollection,
    ) -> Result<(ResolvedItem<'a>, Option<ResolvedItem<'a>>), UnknownPath> {
        let path: Vec<_> = self
            .segments
            .iter()
            .map(|path_segment| path_segment.ident.to_string())
            .collect();
        let ty = krate_collection
            .get_item_by_resolved_path(&path, &self.package_id)
            // TODO: Remove this unwrap
            .unwrap()
            .map_err(|e| UnknownPath(self.to_owned(), e.into()))?;
        let qself_ty = match &self.qualified_self {
            None => None,
            Some(ResolvedPathQualifiedSelf { path, .. }) => {
                let segments: Vec<_> = path
                    .segments
                    .iter()
                    .map(|path_segment| path_segment.ident.to_string())
                    .collect();
                let ty = krate_collection
                    .get_item_by_resolved_path(&segments, &self.package_id)
                    // TODO: Remove this unwrap
                    .unwrap()
                    .map_err(|e| UnknownPath(path.deref().to_owned(), e.into()))?;
                Some(ty)
            }
        };
        Ok((ty, qself_ty))
    }

    pub fn render_path(&self, id2name: &BiHashMap<&PackageId, String>) -> String {
        let mut buffer = String::new();
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
            write!(&mut buffer, "<{} as ", qself.path.render_path(id2name)).unwrap();
            qself_closing_wedge_index = Some(qself.position);
        }
        write!(&mut buffer, "{}", crate_name).unwrap();
        for (index, path_segment) in self.segments[1..].iter().enumerate() {
            write!(&mut buffer, "::{}", path_segment.ident).unwrap();
            let generic_arguments = &path_segment.generic_arguments;
            if !generic_arguments.is_empty() {
                write!(&mut buffer, "::<").unwrap();
                let mut arguments = generic_arguments.iter().peekable();
                while let Some(argument) = arguments.next() {
                    write!(&mut buffer, "{}", argument.render_path(id2name)).unwrap();
                    if arguments.peek().is_some() {
                        write!(&mut buffer, ", ").unwrap();
                    }
                }
                write!(&mut buffer, ">").unwrap();
            }
            if Some(index + 1) == qself_closing_wedge_index {
                write!(&mut buffer, ">").unwrap();
            }
        }
        buffer
    }
}

impl Display for ResolvedPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let last_segment_index = self.segments.len().saturating_sub(1);
        let mut qself_closing_wedge_index = None;
        if let Some(qself) = &self.qualified_self {
            write!(f, "<{} as ", qself.path)?;
            qself_closing_wedge_index = Some(qself.position);
        }
        for (i, segment) in self.segments.iter().enumerate() {
            write!(f, "{}", segment)?;
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

impl Display for ResolvedPathSegment {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.ident)?;
        if !self.generic_arguments.is_empty() {
            write!(f, "::<")?;
        }
        let last_argument_index = self.generic_arguments.len().saturating_sub(1);
        for (j, argument) in self.generic_arguments.iter().enumerate() {
            write!(f, "{}", argument)?;
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

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error(transparent)]
    InvalidPath(#[from] InvalidCallPath),
    #[error(transparent)]
    PathMustBeAbsolute(#[from] PathMustBeAbsolute),
}

impl ParseError {
    pub(crate) fn raw_identifiers(&self) -> &RawCallableIdentifiers {
        match self {
            ParseError::InvalidPath(e) => &e.raw_identifiers,
            ParseError::PathMustBeAbsolute(e) => &e.raw_identifiers,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub struct PathMustBeAbsolute {
    pub(crate) raw_identifiers: RawCallableIdentifiers,
    pub(crate) relative_path: String,
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

#[derive(thiserror::Error, Debug)]
pub struct UnknownPath(pub ResolvedPath, #[source] anyhow::Error);

impl Display for UnknownPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let path = &self.0;
        let krate = path.crate_name().to_string();
        write!(
            f,
            "I could not find '{path}' in the auto-generated documentation for '{krate}'"
        )
    }
}
