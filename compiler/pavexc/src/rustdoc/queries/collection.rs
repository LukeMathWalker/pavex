use std::borrow::Cow;
use std::sync::Arc;

use rustdoc_types::{Item, ItemEnum};

use rustdoc_resolver::{GenericBindings, resolve_type};
use crate::language::{
    FQGenericArgument, FQPathType, UnknownCrate, krate2package_id, resolve_fq_path_type,
};
use crate::rustdoc::CannotGetCrateData;
use rustdoc_ext::RustdocKindExt;

use super::super::AnnotatedItem;
use super::super::annotations::AnnotationCoordinates;
use super::super::indexer::PavexIndexer;
use super::super::progress_reporter::ShellProgress;
use super::CrateCollection;
use rustdoc_processor::queries::Crate;
use rustdoc_processor::{GlobalItemId, UnknownItemPath};

/// Extension trait adding Pavex-specific methods to `CrateCollection`.
pub trait CrateCollectionExt {
    /// Convenience constructor for Pavex: creates a `PavexIndexer` and the
    /// pre-configured on-disk cache, then returns a ready-to-use collection.
    fn new_pavex(
        toolchain_name: String,
        package_graph: guppy::graph::PackageGraph,
        project_fingerprint: String,
        cache_workspace_package_docs: bool,
        diagnostic_sink: crate::diagnostic::DiagnosticSink,
    ) -> Result<CrateCollection, anyhow::Error>;

    /// Retrieve the annotation associated with the given item, if any.
    fn annotation(&self, item_id: &GlobalItemId) -> Option<&AnnotatedItem>;

    /// Retrieve the annotation that these coordinates point to, if any.
    #[allow(clippy::type_complexity)]
    fn annotation_for_coordinates(
        &self,
        c: &AnnotationCoordinates,
    ) -> Result<Result<Option<(&Crate, &AnnotatedItem)>, UnknownCrate>, CannotGetCrateData>;

    fn get_type_by_resolved_path(
        &self,
        resolved_path: crate::language::FQPath,
    ) -> Result<
        Result<(crate::language::FQPath, ResolvedItem<'_>), GetItemByResolvedPathError>,
        CannotGetCrateData,
    >;
}

impl CrateCollectionExt for CrateCollection {
    fn new_pavex(
        toolchain_name: String,
        package_graph: guppy::graph::PackageGraph,
        project_fingerprint: String,
        cache_workspace_package_docs: bool,
        diagnostic_sink: crate::diagnostic::DiagnosticSink,
    ) -> Result<CrateCollection, anyhow::Error> {
        let disk_cache = super::super::cache::pavex_rustdoc_cache(
            &toolchain_name,
            cache_workspace_package_docs,
            &package_graph,
        )?;
        let indexer = PavexIndexer::new(diagnostic_sink);
        Ok(CrateCollection::new(
            indexer,
            toolchain_name,
            package_graph,
            project_fingerprint,
            disk_cache,
            Box::new(ShellProgress),
        ))
    }

    fn annotation(&self, item_id: &GlobalItemId) -> Option<&AnnotatedItem> {
        self.get_annotated_items(&item_id.package_id)?
            .get_by_item_id(item_id.rustdoc_item_id)
    }

    #[allow(clippy::type_complexity)]
    fn annotation_for_coordinates(
        &self,
        c: &AnnotationCoordinates,
    ) -> Result<Result<Option<(&Crate, &AnnotatedItem)>, UnknownCrate>, CannotGetCrateData> {
        let package_id = match krate2package_id(
            &c.created_at.package_name,
            &c.created_at.package_version,
            self.package_graph(),
        ) {
            Ok(p) => p,
            Err(e) => return Ok(Err(e)),
        };
        let krate = self.get_or_compute(&package_id)?;
        let annotations = self.get_annotated_items(&package_id);
        Ok(Ok(annotations
            .and_then(|a| a.get_by_annotation_id(&c.id))
            .map(|item| (krate, item))))
    }

    fn get_type_by_resolved_path(
        &self,
        mut resolved_path: crate::language::FQPath,
    ) -> Result<
        Result<(crate::language::FQPath, ResolvedItem<'_>), GetItemByResolvedPathError>,
        CannotGetCrateData,
    > {
        let mut path_without_generics = resolved_path
            .segments
            .iter()
            .map(|p| p.ident.clone())
            .collect::<Vec<_>>();
        let krate = self.get_or_compute(&resolved_path.package_id)?;
        // The path may come from a crate that depends on the one we are re-examining
        // but with a rename in its `Cargo.toml`. We normalize the path to the original crate name
        // in order to get a match in the index.
        path_without_generics[0] = krate.crate_name();

        let Ok(mut type_id) = krate.get_item_id_by_path(&path_without_generics, self)? else {
            return Ok(Err(UnknownItemPath {
                path: path_without_generics,
            }
            .into()));
        };

        let mut item = self.get_item_by_global_type_id(&type_id);

        if !matches!(
            item.inner,
            ItemEnum::Struct(_)
                | ItemEnum::Enum(_)
                | ItemEnum::TypeAlias(_)
                | ItemEnum::Trait(_)
                | ItemEnum::Primitive(_)
        ) {
            return Ok(Err(GetItemByResolvedPathError::UnsupportedItemKind(
                UnsupportedItemKind {
                    path: path_without_generics,
                    kind: item.inner.kind().into(),
                },
            )));
        }

        // We eagerly check if the item is an alias, and if so we follow it
        // to the original type.
        // This process might take multiple iterations, since the alias might point to another
        // alias, recursively.
        let mut krate = self.get_or_compute(&type_id.package_id)?;
        loop {
            let ItemEnum::TypeAlias(type_alias) = &item.inner else {
                break;
            };
            let rustdoc_types::Type::ResolvedPath(aliased_path) = &type_alias.type_ else {
                break;
            };

            // The aliased type might be a re-export of a foreign type,
            // therefore we go through the summary here rather than
            // going straight for a local id lookup.
            let aliased_summary = krate
                .get_summary_by_local_type_id(&aliased_path.id)
                .unwrap();
            let aliased_package_id = krate
                .compute_package_id_for_crate_id(aliased_summary.crate_id, self)
                .map_err(|e| CannotGetCrateData {
                    package_spec: aliased_summary.crate_id.to_string(),
                    source: Arc::new(e),
                })?;
            let aliased_krate = self.get_or_compute(&aliased_package_id)?;
            let Ok(aliased_type_id) =
                aliased_krate.get_item_id_by_path(&aliased_summary.path, self)?
            else {
                return Ok(Err(UnknownItemPath {
                    path: aliased_summary.path.clone(),
                }
                .into()));
            };
            let aliased_item = self.get_item_by_global_type_id(&aliased_type_id);

            let new_path = {
                let path_args = &resolved_path.segments.last().unwrap().generic_arguments;
                let alias_generics = &type_alias.generics.params;
                let mut name2path_arg = GenericBindings::default();
                for (path_arg, alias_generic) in path_args.iter().zip(alias_generics.iter()) {
                    match path_arg {
                        FQGenericArgument::Type(t) => {
                            let t = resolve_fq_path_type(t, self).unwrap();
                            name2path_arg.types.insert(alias_generic.name.clone(), t);
                        }
                        FQGenericArgument::Lifetime(l) => {
                            name2path_arg
                                .lifetimes
                                .insert(alias_generic.name.clone(), l.to_binding_name());
                        }
                    }
                }

                let aliased = resolve_type(
                    &type_alias.type_,
                    type_id.package_id(),
                    self,
                    &name2path_arg,
                )
                .unwrap();
                let aliased: FQPathType = aliased.into();
                let FQPathType::ResolvedPath(aliased_path) = aliased else {
                    unreachable!();
                };
                (*aliased_path.path).clone()
            };

            // Update the loop variables to reflect alias resolution.
            type_id = aliased_type_id;
            item = aliased_item;
            krate = aliased_krate;
            resolved_path = new_path;
        }

        let resolved_item = ResolvedItem {
            item,
            item_id: type_id,
        };
        Ok(Ok((resolved_path, resolved_item)))
    }
}

#[derive(Debug, Clone)]
pub struct ResolvedItem<'a> {
    pub item: Cow<'a, Item>,
    pub item_id: GlobalItemId,
}

#[derive(thiserror::Error, Debug)]
pub enum GetItemByResolvedPathError {
    #[error(transparent)]
    UnknownItemPath(UnknownItemPath),
    #[error(transparent)]
    UnsupportedItemKind(UnsupportedItemKind),
}

impl From<UnsupportedItemKind> for GetItemByResolvedPathError {
    fn from(value: UnsupportedItemKind) -> Self {
        Self::UnsupportedItemKind(value)
    }
}

impl From<UnknownItemPath> for GetItemByResolvedPathError {
    fn from(value: UnknownItemPath) -> Self {
        Self::UnknownItemPath(value)
    }
}

#[derive(thiserror::Error, Debug)]
pub struct UnsupportedItemKind {
    pub path: Vec<String>,
    pub kind: String,
}

impl std::fmt::Display for UnsupportedItemKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let path = self.path.join("::").replace(' ', "");
        write!(
            f,
            "'{path}' pointed at {} item. I don't know how to handle that (yet)",
            self.kind
        )
    }
}
