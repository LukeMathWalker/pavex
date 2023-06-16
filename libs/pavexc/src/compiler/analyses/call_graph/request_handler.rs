use guppy::graph::PackageGraph;

use pavex::blueprint::constructor::Lifecycle;

use crate::compiler::analyses::call_graph::borrow_checker::OrderedCallGraph;
use crate::compiler::analyses::call_graph::{
    core_graph::build_call_graph, CallGraph, NumberOfAllowedInvocations,
};
use crate::compiler::analyses::components::{ComponentDb, ComponentId};
use crate::compiler::analyses::computations::ComputationDb;
use crate::compiler::analyses::constructibles::ConstructibleDb;
use crate::rustdoc::CrateCollection;

/// Build a [`CallGraph`] for a request handler.
#[tracing::instrument(name = "Compute handler call graph", skip_all)]
pub(crate) fn handler_call_graph(
    request_handler: ComponentId,
    computation_db: &mut ComputationDb,
    component_db: &mut ComponentDb,
    constructible_db: &ConstructibleDb,
    package_graph: &PackageGraph,
    krate_collection: &CrateCollection,
    diagnostics: &mut Vec<miette::Error>,
) -> Result<OrderedCallGraph, ()> {
    fn lifecycle2invocations(l: &Lifecycle) -> Option<NumberOfAllowedInvocations> {
        match l {
            Lifecycle::Singleton => None,
            Lifecycle::RequestScoped => Some(NumberOfAllowedInvocations::One),
            Lifecycle::Transient => Some(NumberOfAllowedInvocations::Multiple),
        }
    }
    let Ok(CallGraph {
        call_graph,
        root_node_index,
        root_scope_id,
    }) = build_call_graph(
        request_handler,
        computation_db,
        component_db,
        constructible_db,
        lifecycle2invocations,
        diagnostics,
    ) else {
        return Err(());
    };

    OrderedCallGraph::new(
        CallGraph {
            call_graph,
            root_node_index,
            root_scope_id,
        },
        component_db,
        computation_db,
        package_graph,
        krate_collection,
        diagnostics,
    )
}
