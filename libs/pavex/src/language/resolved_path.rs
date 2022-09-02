use std::fmt::{Display, Formatter};
use std::hash::Hash;

use guppy::PackageId;
use itertools::Itertools;
use quote::format_ident;
use rustdoc_types::{Item, ItemEnum};
use syn::ExprPath;

use pavex_builder::RawCallableIdentifiers;

use crate::language::{CallPath, InvalidCallPath};
use crate::rustdoc::CrateCollection;

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
#[derive(Clone, Debug, Hash, Eq)]
pub(crate) struct ResolvedPath {
    pub path: CallPath,
    pub package_id: PackageId,
}

impl PartialEq for ResolvedPath {
    fn eq(&self, other: &Self) -> bool {
        let is_equal = self.package_id == other.package_id
            && self.path.0.attrs == other.path.0.attrs
            && self.path.0.qself == other.path.0.qself
            && self.path.0.path.segments.len() == other.path.0.path.segments.len();
        if is_equal {
            // We want to ignore the first segment of the path, because dependencies can be
            // renamed and this can lead to equivalent paths not being considered equal.
            // Given that we already have the package id as part of the type, it is safe
            // to disregard the first segment when determining equality.
            let self_segments = self.path.0.path.segments.iter().skip(1);
            let other_segments = other.path.0.path.segments.iter().skip(1);
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

impl ResolvedPath {
    pub fn parse(
        identifiers: &RawCallableIdentifiers,
        graph: &guppy::graph::PackageGraph,
    ) -> Result<Self, ParseError> {
        let mut path = CallPath::parse(identifiers)?;
        // Normalize by removing the leading semicolon for external paths.
        path.0.path.leading_colon = None;
        if path.leading_path_segment() == "crate" {
            let first_segment = path
                .0
                .path
                .segments
                .first_mut()
                .expect("Bug: an `ExprPath` with no path segments!");
            // Unwrapping here is safe: there is always at least one path segment in a successfully
            // parsed `ExprPath`.
            first_segment.ident = format_ident!("{}", identifiers.registered_at());
        }

        let registration_crate_name = identifiers.registered_at();
        let registration_package = graph.packages()
            .find(|p| p.name() == registration_crate_name)
            .expect("There is no package in the current workspace whose name matches the registration crate for these identifiers");
        let krate_name_candidate = path.leading_path_segment().to_string();
        let package_id = if registration_package.name() == krate_name_candidate {
            registration_package.id().to_owned()
        } else if let Some(dependency) = registration_package
            .direct_links()
            .find(|d| d.resolved_name() == krate_name_candidate)
        {
            dependency.to().id().to_owned()
        } else {
            return Err(PathMustBeAbsolute {
                raw_identifiers: identifiers.to_owned(),
            }
            .into());
        };
        Ok(Self { path, package_id })
    }

    /// Return the name of the crate that this type belongs to.
    ///
    /// E.g. `my_crate::my_module::MyType` will return `my_crate`.
    pub fn crate_name(&self) -> &proc_macro2::Ident {
        self.path.leading_path_segment()
    }

    /// Find information about the type that the path points at.
    pub fn find_type(&self, krate_collection: &mut CrateCollection) -> Result<Item, UnknownPath> {
        // TODO: remove unwrap here
        let krate = krate_collection
            .get_or_compute_by_id(&self.package_id)
            .unwrap();
        let path_segments: Vec<_> = self
            .path
            .0
            .path
            .segments
            .iter()
            .map(|path_segment| path_segment.ident.to_string())
            .collect();
        if let Ok(t) = krate.get_type_by_path(&path_segments) {
            return Ok(t.to_owned());
        }
        // The path might be pointing to a method, which is not a type.
        // We drop the last segment to see if we can get a hit on the struct/enum type
        // to which the method belongs.
        if path_segments.len() < 3 {
            // It has to be at least three segments - crate name, type name, method name.
            // If it's shorter than three, it's just an unknown path.
            return Err(UnknownPath(self.to_owned().into()));
        }
        let (method_name, type_path_segments) = path_segments.split_last().unwrap();
        if let Ok(t) = krate.get_type_by_path(type_path_segments) {
            let impl_block_ids = match &t.inner {
                ItemEnum::Struct(s) => &s.impls,
                ItemEnum::Enum(enum_) => &enum_.impls,
                _ => return Err(UnknownPath(self.to_owned().into())),
            };
            for impl_block_id in impl_block_ids {
                let impl_block = krate.get_type_by_id(impl_block_id);
                if let ItemEnum::Impl(impl_block) = &impl_block.inner {
                    for impl_item_id in &impl_block.items {
                        let impl_item = krate.get_type_by_id(impl_item_id);
                        if impl_item.name.as_ref() == Some(method_name) {
                            if let ItemEnum::Method(_) = &impl_item.inner {
                                return Ok(impl_item.to_owned());
                            }
                        }
                    }
                } else {
                    unreachable!()
                }
            }
        }
        Err(UnknownPath(self.to_owned().into()))
    }
}

impl Display for ResolvedPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.path)
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ParseError {
    #[error(transparent)]
    InvalidPath(#[from] InvalidCallPath),
    #[error(transparent)]
    PathMustBeAbsolute(#[from] PathMustBeAbsolute),
}

#[derive(Debug, thiserror::Error)]
pub(crate) struct PathMustBeAbsolute {
    pub raw_identifiers: RawCallableIdentifiers,
}

impl Display for PathMustBeAbsolute {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let path = self.raw_identifiers.raw_path();
        write!(f, "`{path}` is not a fully-qualified import path.")
    }
}

impl AsRef<ExprPath> for ResolvedPath {
    fn as_ref(&self) -> &ExprPath {
        self.path.as_ref()
    }
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
/// Why does this type even exist?
///
/// It turns out that most of `syn`'s types are neither `Send` nor `Sync` because, deep down, they
/// are holding on to one or more `Span`s, which are not `Send` nor `Sync`.
///
/// As a consequence, we can't store `syn`'s types as fields on our error types, because `miette`
/// expects the source error types to be `Send` and `Sync`.
///
/// This type is a crutch - every time we need to store a [`ResolvedPath`] in an error
/// type, we encode it as a string. By wrapping it in a new type, we make it obvious what that
/// string means and we keep track of the fact that this can be infallibly deserialized back
/// into a `syn::ExprPath`.
pub(crate) struct EncodedResolvedPath(String, PackageId);

impl EncodedResolvedPath {
    pub fn decode(&self) -> ResolvedPath {
        ResolvedPath {
            path: CallPath(syn::parse_str(&self.0).unwrap()),
            package_id: self.1.clone(),
        }
    }
}

impl std::fmt::Display for EncodedResolvedPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let decoded = self.decode();
        std::fmt::Display::fmt(&decoded, f)
    }
}

impl From<ResolvedPath> for EncodedResolvedPath {
    fn from(p: ResolvedPath) -> Self {
        Self(p.path.to_string(), p.package_id)
    }
}

#[derive(thiserror::Error, Debug)]
pub(crate) struct UnknownPath(pub EncodedResolvedPath);

impl Display for UnknownPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let decoded = self.0.decode();
        let krate = decoded.crate_name().to_string();
        write!(
            f,
            "I could not find '{decoded}' in the auto-generated documentation for '{krate}'"
        )
    }
}
