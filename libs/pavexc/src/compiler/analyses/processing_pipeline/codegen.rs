use crate::compiler::analyses::components::ComponentDb;
use crate::compiler::analyses::computations::ComputationDb;
use crate::compiler::analyses::processing_pipeline::RequestHandlerPipeline;
use bimap::BiHashMap;
use guppy::PackageId;
use syn::ItemFn;

impl RequestHandlerPipeline {
    pub(crate) fn codegen(
        &self,
        package_id2name: &BiHashMap<PackageId, String>,
        component_db: &ComponentDb,
        computation_db: &ComputationDb,
    ) -> Result<ItemFn, anyhow::Error> {
        let fn_ = self
            .handler_call_graph
            .codegen(package_id2name, component_db, computation_db)?;
        Ok(fn_)
    }
}
