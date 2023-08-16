use crate::compiler::analyses::call_graph::{
    request_scoped_call_graph, CallGraph, CallGraphNode, InputParameterSource, OrderedCallGraph,
    RawCallGraph,
};
use crate::compiler::analyses::components::{ComponentDb, ComponentId};
use crate::compiler::analyses::computations::ComputationDb;
use crate::compiler::analyses::constructibles::ConstructibleDb;
use crate::rustdoc::CrateCollection;
use ahash::{HashMap, HashMapExt};
use guppy::graph::PackageGraph;
use indexmap::{IndexMap, IndexSet};
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
        let mut middleware_call_graphs = IndexMap::with_capacity(middleware_ids.len());
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

            middleware_call_graphs.insert(middleware_id, middleware_call_graph);
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

        // Step 3a: For each middleware, determine which request-scoped components must be actually
        //   passed down through the `Next<_>` parameter to the downstream stages of the pipeline.
        //
        // In order to pull this off, we walk the chain in reverse order and accumulate the set of
        // request-scoped components that are expected as input.
        let mut next_field_types: IndexSet<ComponentId> = IndexSet::new();
        extract_request_scoped_inputs(
            &handler_call_graph.call_graph,
            &component_db,
            &mut next_field_types,
        );
        let mut middleware_id2next_field_types: HashMap<ComponentId, IndexSet<ComponentId>> =
            HashMap::new();
        for (middleware_id, middleware_call_graph) in middleware_call_graphs.iter().rev() {
            middleware_id2next_field_types.insert(*middleware_id, next_field_types.clone());

            // Remove all the request-scoped components initialised by this middleware from the set.
            // They can't be needed upstream since they were initialised here!
            for node in middleware_call_graph.call_graph.node_weights() {
                if let CallGraphNode::Compute { component_id, .. } = node {
                    if component_db.lifecycle(*component_id) == Some(&Lifecycle::RequestScoped) {
                        next_field_types.remove(component_id);
                    }
                }
            }

            // But this middleware can in turn ask for some request-scoped components to be passed
            // down from the upstream stages of the pipeline, therefore we need to add those to
            // the set.
            extract_request_scoped_inputs(
                &middleware_call_graph.call_graph,
                &component_db,
                &mut next_field_types,
            );
        }

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

/// Extract the set of request-scoped components that are used as inputs in the provided call graph.
///
/// The extracted component ids are inserted into the provided buffer set.
fn extract_request_scoped_inputs(
    call_graph: &RawCallGraph,
    component_db: &ComponentDb,
    buffer: &mut IndexSet<ComponentId>,
) {
    for node in call_graph.node_weights() {
        let CallGraphNode::InputParameter { source, .. } = node else { continue; };
        let InputParameterSource::Component(component_id) = source else { continue; };
        if component_db.lifecycle(*component_id) == Some(&Lifecycle::RequestScoped) {
            buffer.insert(*component_id);
        }
    }
}
