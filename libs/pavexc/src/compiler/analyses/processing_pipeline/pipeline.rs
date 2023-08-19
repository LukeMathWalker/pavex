use crate::compiler::analyses::call_graph::{
    request_scoped_call_graph, CallGraphNode, InputParameterSource, OrderedCallGraph, RawCallGraph,
};
use crate::compiler::analyses::components::{ComponentDb, ComponentId, HydratedComponent};
use crate::compiler::analyses::computations::ComputationDb;
use crate::compiler::analyses::constructibles::ConstructibleDb;
use crate::compiler::analyses::processing_pipeline::graph_iter::PipelineGraphIterator;
use crate::compiler::app::GENERATED_APP_PACKAGE_ID;
use crate::language::{Callable, InvocationStyle, PathType};
use crate::rustdoc::CrateCollection;
use ahash::{HashMap, HashMapExt};
use guppy::graph::PackageGraph;
use guppy::PackageId;
use indexmap::{IndexMap, IndexSet};
use pavex::blueprint::constructor::{CloningStrategy, Lifecycle};
use std::collections::BTreeMap;

/// A request handler pipeline is the combination of a root compute node (i.e. the request handler)
/// and an ordered sequence of wrapping middlewares ahead of it, feeding into each other.
pub(crate) struct RequestHandlerPipeline {
    pub(crate) handler_id: ComponentId,
    pub(crate) handler_call_graph: OrderedCallGraph,
    pub(crate) middleware_id2stage_data: IndexMap<ComponentId, (OrderedCallGraph, PathType)>,
}

impl RequestHandlerPipeline {
    /// Build a [`RequestHandlerPipeline`] for the request handler with the provided [`ComponentId`].
    pub(crate) fn new(
        handler_id: ComponentId,
        mut computation_db: &mut ComputationDb,
        mut component_db: &mut ComponentDb,
        constructible_db: &mut ConstructibleDb,
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

        // TODO: borrow-checker
        // Step X: Check that the entire pipeline satisfies the constraints imposed by the
        //   borrow-checker.
        //   Determine, for each middleware, if they consume a request-scoped component
        //   that is also needed by later stages of the pipeline.

        // Since we now know which request-scoped components are needed by each middleware, we can
        // now make the call graph for each middleware concrete—i.e. we can replace the generic
        // `Next<_>` parameter with a concrete type (that we will codegen later on).
        let mut middleware_id2stage_data: IndexMap<ComponentId, (OrderedCallGraph, PathType)> =
            IndexMap::new();
        for (i, (middleware_id, next_state_types)) in
            middleware_id2next_field_types.iter().enumerate()
        {
            let next_state_bindings = next_state_types
                .iter()
                .enumerate()
                .map(|(i, component_id)| {
                    let component =
                        component_db.hydrated_component(*component_id, &mut computation_db);
                    let type_ = component.output_type();
                    // TODO: naming can be improved here.
                    (format!("rs_{i}"), type_.to_owned())
                })
                .collect::<BTreeMap<_, _>>();
            let next_state_type = PathType {
                package_id: PackageId::new(GENERATED_APP_PACKAGE_ID),
                rustdoc_id: None,
                // TODO: we should put everything in a sub-module specific to the request handler
                //   that we're currently processing.
                base_type: vec!["crate".into(), format!("Next{i}")],
                generic_arguments: vec![],
            };

            // We register a constructor, in order to make it possible to build an instance of
            // `next_type`.
            let next_state_constructor = Callable {
                is_async: false,
                path: next_state_type.resolved_path(),
                output: Some(next_state_type.clone().into()),
                inputs: next_state_bindings.values().cloned().collect(),
                invocation_style: InvocationStyle::StructLiteral {
                    field_names: next_state_bindings,
                },
                source_coordinates: None,
            };
            let next_state_callable_id = computation_db.get_or_intern(next_state_constructor);
            let next_state_scope_id = component_db.scope_id(*middleware_id);
            let next_state_constructor_id = component_db
                .get_or_intern_constructor(
                    next_state_callable_id,
                    Lifecycle::RequestScoped,
                    next_state_scope_id,
                    CloningStrategy::NeverClone,
                    computation_db,
                )
                .unwrap();
            constructible_db.insert(next_state_constructor_id, component_db, computation_db);

            // Since we now have the concrete type of the generic in `Next<_>`, we can bind
            // the generic type parameter of the middleware to that concrete type.
            let HydratedComponent::WrappingMiddleware(mw) = component_db.hydrated_component(*middleware_id, computation_db) else {
                unreachable!()
            };
            let next_input = &mw.input_types()[mw.next_input_index()];
            let next_generic_parameters = next_input.unassigned_generic_type_parameters();

            #[cfg(debug_assertions)]
            assert_eq!(
                next_generic_parameters.len(),
                1,
                "Next<_> should have exactly one unassigned generic type parameter"
            );

            let next_generic_parameter = next_generic_parameters.iter().next().unwrap().to_owned();

            let mut bindings = HashMap::with_capacity(1);
            bindings.insert(next_generic_parameter, next_state_type.clone().into());
            let bound_middleware_id = component_db.bind_generic_type_parameters(
                *middleware_id,
                &bindings,
                computation_db,
            );

            let HydratedComponent::WrappingMiddleware(bound_mw) = component_db.hydrated_component(bound_middleware_id, computation_db) else {
                unreachable!()
            };
            // Force the constructibles database to bind a constructor for `Next<{NextState}>`.
            // Really ugly, but alas.
            assert!(constructible_db
                .get_or_try_bind(
                    next_state_scope_id,
                    &bound_mw.next_input_type().to_owned(),
                    component_db,
                    computation_db,
                )
                .is_some());

            // We can now build the call graph for the middleware, using the concrete type of
            // `Next<_>` as the input type.
            let middleware_call_graph = request_scoped_call_graph(
                bound_middleware_id,
                &request_scoped_prebuilt_ids,
                &mut computation_db,
                &mut component_db,
                &constructible_db,
                &package_graph,
                &krate_collection,
                &mut diagnostics,
            )?;
            middleware_id2stage_data
                .insert(*middleware_id, (middleware_call_graph, next_state_type));
        }

        Ok(Self {
            handler_id,
            handler_call_graph,
            middleware_id2stage_data,
        })
    }
}

impl RequestHandlerPipeline {
    /// Iterate over all the call graphs in the pipeline, in execution order (middlewares first,
    /// request handler last).
    pub(crate) fn graph_iter(&self) -> PipelineGraphIterator {
        PipelineGraphIterator {
            pipeline: self,
            current_stage: Some(0),
        }
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
