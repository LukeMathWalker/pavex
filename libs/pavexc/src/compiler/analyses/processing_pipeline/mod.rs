use crate::compiler::analyses::call_graph::OrderedCallGraph;
use crate::compiler::analyses::components::{ComponentDb, ComponentId};
use crate::compiler::analyses::computations::ComputationDb;
use crate::compiler::analyses::constructibles::ConstructibleDb;
use crate::rustdoc::CrateCollection;
use guppy::graph::PackageGraph;

/// A processing pipeline is the combination of a root computation (e.g. a request handler) and
/// an ordered sequence of wrapping middlewares ahead of it, feeding into each other.
///
/// For each stage in the pipeline, we will generate an ordered call graph, and then combine them
/// together to form the call graph of the pipeline as a whole.
pub(crate) struct ProcessingPipeline {
    stages: Vec<OrderedCallGraph>,
}

pub(crate) struct RequestHandlerPipeline {
    handler_id: ComponentId,
    pipeline: ProcessingPipeline,
}

impl RequestHandlerPipeline {
    pub(crate) fn new(
        _handler_id: ComponentId,
        _computation_db: &mut ComputationDb,
        _component_db: &mut ComponentDb,
        _constructible_db: &ConstructibleDb,
        _package_graph: &PackageGraph,
        _krate_collection: &CrateCollection,
        _diagnostics: &mut Vec<miette::Error>,
    ) -> Self {
        // Step 1: Determine the sequence of middlewares that the request handler is wrapped in.
        // Step 2: For each middleware, build an ordered call graph.
        // Step 3: Combine the ordered call graphs together.
        // Step 3a: Determine, for each middleware, if they consume a request-scoped component
        //   that is also needed by later stages of the pipeline.
        // Step 3b: If that's the case, insert a "cloning" node into the call graph ahead of
        //   using that component to build the `Next` container for that middleware stage.
        //   If the component is not cloneable, emit a diagnostic and abort.
        todo!()
    }
}
