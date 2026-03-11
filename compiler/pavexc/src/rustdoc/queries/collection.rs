use crate::language::UnknownCrate;
use crate::language::krate2package_id;
use crate::rustdoc::CannotGetCrateData;

use super::super::AnnotatedItem;
use super::super::annotations::AnnotationCoordinates;
use super::super::indexer::PavexIndexer;
use super::super::progress_reporter::ShellProgress;
use super::CrateCollection;
use rustdoc_processor::queries::Crate;
use rustdoc_processor::GlobalItemId;

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
}
