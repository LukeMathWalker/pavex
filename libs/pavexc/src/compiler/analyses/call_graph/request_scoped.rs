use indexmap::IndexSet;
use petgraph::Direction;
use std::collections::BTreeSet;
use tracing::Level;

use pavex_bp_schema::Lifecycle;

use crate::compiler::analyses::call_graph::borrow_checker::OrderedCallGraph;
use crate::compiler::analyses::call_graph::{
    CallGraph, CallGraphNode, NumberOfAllowedInvocations, core_graph::build_call_graph,
};
use crate::compiler::analyses::components::{
    ComponentDb, ComponentId, ConsumptionMode, InsertTransformer,
};
use crate::compiler::analyses::computations::ComputationDb;
use crate::compiler::analyses::constructibles::ConstructibleDb;
use crate::compiler::computation::Computation;
use crate::language::{Callable, InvocationStyle, ResolvedPath, ResolvedPathSegment, ResolvedType};
use crate::rustdoc::CrateCollection;

/// Build an [`OrderedCallGraph`] for a computation that gets trigger on a per-request basis
/// (e.g. a request handler or a middleware).
#[tracing::instrument(name = "Compute request-scoped ordered call graph", skip_all)]
pub(crate) fn request_scoped_ordered_call_graph(
    root_component_id: ComponentId,
    // The set of request-scoped components that have already been initialised in the upstream
    // stages of the pipeline.
    request_scoped_prebuilt_ids: &IndexSet<ComponentId>,
    error_observer_ids: &[ComponentId],
    computation_db: &mut ComputationDb,
    component_db: &mut ComponentDb,
    constructible_db: &ConstructibleDb,
    krate_collection: &CrateCollection,
    diagnostics: &mut crate::diagnostic::DiagnosticSink,
) -> Result<OrderedCallGraph, ()> {
    let Ok(CallGraph {
        call_graph,
        root_node_index,
        root_scope_id,
    }) = request_scoped_call_graph(
        root_component_id,
        request_scoped_prebuilt_ids,
        error_observer_ids,
        computation_db,
        component_db,
        constructible_db,
        diagnostics,
    )
    else {
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
        krate_collection,
        diagnostics,
    )
}

/// Build an [`CallGraph`] for a computation that gets trigger on a per-request basis
/// (e.g. a request handler or a middleware).
pub(crate) fn request_scoped_call_graph(
    root_component_id: ComponentId,
    // The set of request-scoped components that have already been initialised in the upstream
    // stages of the pipeline.
    request_scoped_prebuilt_ids: &IndexSet<ComponentId>,
    error_observer_ids: &[ComponentId],
    computation_db: &mut ComputationDb,
    component_db: &mut ComponentDb,
    constructible_db: &ConstructibleDb,
    diagnostics: &mut crate::diagnostic::DiagnosticSink,
) -> Result<CallGraph, ()> {
    let mut graph_root = String::new();
    if tracing::enabled!(Level::DEBUG) {
        let component = component_db.hydrated_component(root_component_id, computation_db);
        if let Computation::Callable(c) = component.computation() {
            graph_root = c.path.to_string();
        }
    }
    let span = tracing::debug_span!(
        "Compute request-scoped call graph",
        graph_root = %graph_root,
    );
    let _guard = span.enter();

    let call_graph = _request_scoped_call_graph(
        root_component_id,
        request_scoped_prebuilt_ids,
        error_observer_ids,
        computation_db,
        component_db,
        constructible_db,
        diagnostics,
    )?;
    if component_db.is_pre_processing_middleware(root_component_id) {
        augment_preprocessing_graph(
            call_graph,
            root_component_id,
            request_scoped_prebuilt_ids,
            error_observer_ids,
            computation_db,
            component_db,
            constructible_db,
            diagnostics,
        )
    } else {
        Ok(call_graph)
    }
}

fn _request_scoped_call_graph(
    root_component_id: ComponentId,
    // The set of request-scoped components that have already been initialised in the upstream
    // stages of the pipeline.
    request_scoped_prebuilt_ids: &IndexSet<ComponentId>,
    error_observer_ids: &[ComponentId],
    computation_db: &mut ComputationDb,
    component_db: &mut ComponentDb,
    constructible_db: &ConstructibleDb,
    diagnostics: &mut crate::diagnostic::DiagnosticSink,
) -> Result<CallGraph, ()> {
    fn lifecycle2invocations(l: Lifecycle) -> Option<NumberOfAllowedInvocations> {
        match l {
            Lifecycle::Singleton => None,
            Lifecycle::RequestScoped => Some(NumberOfAllowedInvocations::One),
            Lifecycle::Transient => Some(NumberOfAllowedInvocations::Multiple),
        }
    }
    build_call_graph(
        root_component_id,
        request_scoped_prebuilt_ids,
        error_observer_ids,
        computation_db,
        component_db,
        constructible_db,
        lifecycle2invocations,
        diagnostics,
    )
}

/// Sort out the output nodes of the call graph for a pre-processing middleware.
/// They all need to be of type `Processing`.
#[tracing::instrument(name = "Compute pre-processing middleware call graph", skip_all)]
fn augment_preprocessing_graph(
    call_graph: CallGraph,
    root_component_id: ComponentId,
    // The set of request-scoped components that have already been initialised in the upstream
    // stages of the pipeline.
    request_scoped_prebuilt_ids: &IndexSet<ComponentId>,
    error_observer_ids: &[ComponentId],
    computation_db: &mut ComputationDb,
    component_db: &mut ComponentDb,
    constructible_db: &ConstructibleDb,
    diagnostics: &mut crate::diagnostic::DiagnosticSink,
) -> Result<CallGraph, ()> {
    assert!(component_db.is_pre_processing_middleware(root_component_id));
    // We need to add a transformer to convert all `Response` leaf nodes into
    // `Processing` nodes, via `Processing::EarlyReturn`.
    let output_node_indexes: BTreeSet<_> = call_graph
        .call_graph
        .externals(Direction::Outgoing)
        .collect();
    for output_node_index in output_node_indexes {
        let CallGraphNode::Compute { component_id, .. } = call_graph.call_graph[output_node_index]
        else {
            unreachable!()
        };
        let hydrated_component = component_db.hydrated_component(component_id, computation_db);
        let Some(output_type) = hydrated_component.output_type() else {
            continue;
        };
        if output_type != &component_db.pavex_response {
            continue;
        }

        let ResolvedType::ResolvedPath(processing_path) = component_db.pavex_processing.as_ref()
        else {
            unreachable!()
        };
        let early_return_segments = processing_path
            .base_type
            .iter()
            .cloned()
            .chain(std::iter::once("EarlyReturn".to_string()))
            .map(|s| ResolvedPathSegment {
                ident: s,
                generic_arguments: vec![],
            })
            .collect();
        let wrapper = Callable {
            is_async: false,
            takes_self_as_ref: false,
            output: Some(component_db.pavex_processing.clone()),
            path: ResolvedPath {
                segments: early_return_segments,
                qualified_self: None,
                package_id: processing_path.package_id.clone(),
            },
            inputs: vec![output_type.to_owned()],
            invocation_style: InvocationStyle::FunctionCall,
            source_coordinates: None,
        };
        component_db.get_or_intern_transformer(
            computation_db.get_or_intern(wrapper),
            component_id,
            call_graph.root_scope_id,
            InsertTransformer::Eagerly,
            ConsumptionMode::Move,
            0,
            computation_db,
        );
    }

    // We can now build the call graph again, as we have added the necessary transformers.
    _request_scoped_call_graph(
        root_component_id,
        request_scoped_prebuilt_ids,
        error_observer_ids,
        computation_db,
        component_db,
        constructible_db,
        diagnostics,
    )
}
