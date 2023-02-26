use pavex_builder::Lifecycle;

use crate::web::analyses::call_graph::{build_call_graph, CallGraph, NumberOfAllowedInvocations};
use crate::web::analyses::components::{ComponentDb, ComponentId};
use crate::web::analyses::computations::ComputationDb;
use crate::web::analyses::constructibles::ConstructibleDb;

/// Build a [`CallGraph`] for a request handler.
#[tracing::instrument(name = "compute_handler_call_graph", skip_all)]
pub(crate) fn handler_call_graph(
    request_handler: ComponentId,
    computation_db: &ComputationDb,
    component_db: &ComponentDb,
    constructible_db: &ConstructibleDb,
) -> CallGraph {
    fn lifecycle2invocations(l: &Lifecycle) -> Option<NumberOfAllowedInvocations> {
        match l {
            Lifecycle::Singleton => None,
            Lifecycle::RequestScoped => Some(NumberOfAllowedInvocations::One),
            Lifecycle::Transient => Some(NumberOfAllowedInvocations::Multiple),
        }
    }
    let CallGraph {
        call_graph,
        root_node_index,
    } = build_call_graph(
        request_handler,
        computation_db,
        component_db,
        constructible_db,
        lifecycle2invocations,
    );

    CallGraph {
        call_graph,
        root_node_index,
    }
}
