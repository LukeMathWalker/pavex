use guppy::graph::PackageGraph;
use indexmap::IndexSet;

use pavex::blueprint::constructor::Lifecycle;

use crate::compiler::analyses::call_graph::borrow_checker::OrderedCallGraph;
use crate::compiler::analyses::call_graph::{
    core_graph::build_call_graph, CallGraph, NumberOfAllowedInvocations,
};
use crate::compiler::analyses::components::{ComponentDb, ComponentId};
use crate::compiler::analyses::computations::ComputationDb;
use crate::compiler::analyses::constructibles::ConstructibleDb;
use crate::rustdoc::CrateCollection;

/// Build an [`OrderedCallGraph`] for a computation that gets trigger on a per-request basis
/// (e.g. a request handler or a middleware).
#[tracing::instrument(name = "Compute request-scoped ordered call graph", skip_all)]
pub(crate) fn request_scoped_ordered_call_graph(
    root_component_id: ComponentId,
    // The set of request-scoped components that have already been initialised in the upstream
    // stages of the pipeline.
    request_scoped_prebuilt_ids: &IndexSet<ComponentId>,
    computation_db: &mut ComputationDb,
    component_db: &mut ComponentDb,
    constructible_db: &ConstructibleDb,
    package_graph: &PackageGraph,
    krate_collection: &CrateCollection,
    diagnostics: &mut Vec<miette::Error>,
) -> Result<OrderedCallGraph, ()> {
    let Ok(CallGraph {
        call_graph,
        root_node_index,
        root_scope_id,
    }) = request_scoped_call_graph(
        root_component_id,
        request_scoped_prebuilt_ids,
        computation_db,
        component_db,
        constructible_db,
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

/// Build an [`CallGraph`] for a computation that gets trigger on a per-request basis
/// (e.g. a request handler or a middleware).
#[tracing::instrument(name = "Compute request-scoped call graph", skip_all)]
pub(crate) fn request_scoped_call_graph(
    root_component_id: ComponentId,
    // The set of request-scoped components that have already been initialised in the upstream
    // stages of the pipeline.
    request_scoped_prebuilt_ids: &IndexSet<ComponentId>,
    computation_db: &mut ComputationDb,
    component_db: &mut ComponentDb,
    constructible_db: &ConstructibleDb,
    diagnostics: &mut Vec<miette::Error>,
) -> Result<CallGraph, ()> {
    fn lifecycle2invocations(l: &Lifecycle) -> Option<NumberOfAllowedInvocations> {
        match l {
            Lifecycle::Singleton => None,
            Lifecycle::RequestScoped => Some(NumberOfAllowedInvocations::One),
            Lifecycle::Transient => Some(NumberOfAllowedInvocations::Multiple),
        }
    }
    build_call_graph(
        root_component_id,
        request_scoped_prebuilt_ids,
        computation_db,
        component_db,
        constructible_db,
        lifecycle2invocations,
        diagnostics,
    )
}
