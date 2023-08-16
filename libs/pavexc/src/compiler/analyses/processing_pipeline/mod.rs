use crate::compiler::analyses::call_graph::{
    request_scoped_call_graph, CallGraphNode, OrderedCallGraph,
};
use crate::compiler::analyses::components::{ComponentDb, ComponentId};
use crate::compiler::analyses::computations::ComputationDb;
use crate::compiler::analyses::constructibles::ConstructibleDb;
use crate::rustdoc::CrateCollection;
use guppy::graph::PackageGraph;
use indexmap::IndexSet;
use pavex::blueprint::constructor::Lifecycle;

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
        handler_id: ComponentId,
        mut computation_db: &mut ComputationDb,
        mut component_db: &mut ComponentDb,
        constructible_db: &ConstructibleDb,
        package_graph: &PackageGraph,
        krate_collection: &CrateCollection,
        mut diagnostics: &mut Vec<miette::Error>,
    ) -> Result<Self, ()> {
        // Step 1: Determine the sequence of middlewares that the request handler is wrapped in.
        let middleware_ids = component_db
            .middleware_chain(handler_id)
            .unwrap()
            .to_owned();
        let mut middleware_call_graphs = Vec::with_capacity(middleware_ids.len());
        // Step 2: For each middleware, build an ordered call graph.

        // We need to make sure that request-scoped components are built at most once per request,
        // no matter *where* they are initialised in the overall pipeline.
        // In order to do that, we:
        // - track which request-scoped components are initialised in each middleware in
        //   `request_scoped_ids`
        // - pass `request_scoped_ids` to the downstream stages in the pipeline, to make sure they
        //   don't initialise the same components again but rather reuse the instances created
        //   by the upstream stages.
        let mut request_scoped_prebuilt_ids = IndexSet::new();

        for middleware_id in middleware_ids {
            let middleware_call_graph = request_scoped_call_graph(
                middleware_id,
                &request_scoped_prebuilt_ids,
                &mut computation_db,
                &mut component_db,
                &constructible_db,
                &package_graph,
                &krate_collection,
                &mut diagnostics,
            )?;

            // Add all request-scoped components initialised by this middleware to the set.
            for node in middleware_call_graph.call_graph.node_weights() {
                if let CallGraphNode::Compute { component_id, .. } = node {
                    if component_db.lifecycle(*component_id) == Some(&Lifecycle::RequestScoped) {
                        request_scoped_prebuilt_ids.insert(*component_id);
                    }
                }
            }

            middleware_call_graphs.push(middleware_call_graph);
        }
        let handler_call_graph = request_scoped_call_graph(
            handler_id,
            &request_scoped_prebuilt_ids,
            &mut computation_db,
            &mut component_db,
            &constructible_db,
            &package_graph,
            &krate_collection,
            &mut diagnostics,
        )?;
        // Step 3: Combine the ordered call graphs together.
        // Step 4: Check that the entire pipeline satisfies the constraints imposed by the
        //   borrow-checker.
        // Step 4a: Determine, for each middleware, if they consume a request-scoped component
        //   that is also needed by later stages of the pipeline.
        // Step 4b: If that's the case, insert a "cloning" node into the call graph ahead of
        //   using that component to build the `Next` container for that middleware stage.
        //   If the component is not cloneable, emit a diagnostic and abort.
        todo!()
    }
}
