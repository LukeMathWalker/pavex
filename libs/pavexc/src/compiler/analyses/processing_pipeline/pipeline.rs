use std::collections::BTreeMap;

use ahash::{HashMap, HashMapExt};
use guppy::graph::PackageGraph;
use guppy::PackageId;
use indexmap::{IndexMap, IndexSet};

use pavex_bp_schema::{CloningStrategy, Lifecycle};

use crate::compiler::analyses::call_graph::{
    request_scoped_call_graph, request_scoped_ordered_call_graph, CallGraphNode,
    InputParameterSource, OrderedCallGraph, RawCallGraph,
};
use crate::compiler::analyses::components::{ComponentDb, ComponentId, HydratedComponent};
use crate::compiler::analyses::computations::ComputationDb;
use crate::compiler::analyses::constructibles::ConstructibleDb;
use crate::compiler::analyses::processing_pipeline::graph_iter::PipelineGraphIterator;
use crate::compiler::app::GENERATED_APP_PACKAGE_ID;
use crate::language::{Callable, InvocationStyle, PathType, ResolvedType};
use crate::rustdoc::CrateCollection;

/// A request handler pipeline is the combination of a root compute node (i.e. the request handler)
/// and an ordered sequence of wrapping middlewares ahead of it, feeding into each other.
pub(crate) struct RequestHandlerPipeline {
    /// The name of the local module where the generated types (e.g. `{ConcreteType}` in
    /// `Next<{ConcreteType}>`) will be defined.
    pub(crate) module_name: String,
    pub(crate) handler_call_graph: OrderedCallGraph,
    pub(crate) middleware_id2stage_data: IndexMap<ComponentId, MiddlewareData>,
}

/// Additional per-middleware data that is required to generate code for the over-arching
/// request handler pipeline.
pub(crate) struct MiddlewareData {
    pub(crate) call_graph: OrderedCallGraph,
    pub(crate) next_state: NextState,
}

/// The "state" for `Next<T>` is the concrete type for `T` used in a specific middleware invocation.
///
/// It is computed on a per-pipeline and per-middleware basis, in order to pass down the
/// strict minimum of request-scoped and singleton components that are needed by the downstream
/// stages of the pipeline.
pub(crate) struct NextState {
    pub(crate) type_: PathType,
    /// The state is always a struct.
    /// This map contains the bindings for the fields of the struct: the field name and the type
    /// of the field.
    pub(crate) field_bindings: BTreeMap<String, ResolvedType>,
}

impl RequestHandlerPipeline {
    /// Build a [`RequestHandlerPipeline`] for the request handler with the provided [`ComponentId`].
    pub(crate) fn new(
        handler_id: ComponentId,
        module_name: String,
        computation_db: &mut ComputationDb,
        component_db: &mut ComponentDb,
        constructible_db: &mut ConstructibleDb,
        package_graph: &PackageGraph,
        krate_collection: &CrateCollection,
        diagnostics: &mut Vec<miette::Error>,
    ) -> Result<Self, ()> {
        let error_observer_ids = component_db.error_observers(handler_id).unwrap().to_owned();

        // Step 1: Determine the sequence of middlewares that the request handler is wrapped in.
        let middleware_ids = component_db
            .middleware_chain(handler_id)
            .unwrap()
            .to_owned();

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

        let mut middleware_call_graphs = IndexMap::with_capacity(middleware_ids.len());
        for middleware_id in middleware_ids {
            let middleware_call_graph = request_scoped_call_graph(
                middleware_id,
                &request_scoped_prebuilt_ids,
                &error_observer_ids,
                computation_db,
                component_db,
                constructible_db,
                diagnostics,
            )?;

            // Add all request-scoped components initialised by this middleware to the set.
            extract_request_scoped_compute_nodes(
                &middleware_call_graph.call_graph,
                component_db,
                &mut request_scoped_prebuilt_ids,
            );

            middleware_call_graphs.insert(middleware_id, middleware_call_graph);
        }
        let handler_call_graph = request_scoped_ordered_call_graph(
            handler_id,
            &request_scoped_prebuilt_ids,
            &error_observer_ids,
            computation_db,
            component_db,
            constructible_db,
            package_graph,
            krate_collection,
            diagnostics,
        )?;

        // Step 3: Combine the call graphs together.
        // For each middleware, determine which request-scoped and singleton components
        // must actually be passed down through the `Next<_>` parameter to the downstream
        // stages of the pipeline.
        //
        // In order to pull this off, we walk the chain in reverse order and accumulate the set of
        // request-scoped and singleton components that are expected as input.
        let mut next_field_types: IndexSet<ResolvedType> = IndexSet::new();
        extract_long_lived_inputs(
            &handler_call_graph.call_graph,
            component_db,
            &mut next_field_types,
        );
        let mut middleware_id2next_field_types: IndexMap<ComponentId, IndexSet<ResolvedType>> =
            IndexMap::new();
        for (middleware_id, middleware_call_graph) in middleware_call_graphs.iter().rev() {
            middleware_id2next_field_types.insert(*middleware_id, next_field_types.clone());

            // Remove all the request-scoped components initialised by this middleware from the set.
            // They can't be needed upstream since they were initialised here!
            for node in middleware_call_graph.call_graph.node_weights() {
                if let CallGraphNode::Compute { component_id, .. } = node {
                    if component_db.lifecycle(*component_id) == Some(&Lifecycle::RequestScoped) {
                        let component =
                            component_db.hydrated_component(*component_id, computation_db);
                        if let Some(output_type) = component.output_type() {
                            next_field_types.remove(output_type);
                        }
                    }
                }
            }

            // But this middleware can in turn ask for some long-lived components to be passed
            // down from the upstream stages of the pipeline, therefore we need to add those to
            // the set.
            extract_long_lived_inputs(
                &middleware_call_graph.call_graph,
                component_db,
                &mut next_field_types,
            );
        }
        middleware_id2next_field_types.reverse();

        // Determine, for each middleware, if they consume a request-scoped component
        // that is also needed by later stages of the pipeline.

        // Since we now know which request-scoped components are needed by each middleware, we can
        // make the call graph for each middleware concrete—i.e. we can replace the generic
        // `Next<_>` parameter with a concrete type (that we will codegen later on).
        let mut middleware_id2stage_data = IndexMap::new();
        let mut request_scoped_prebuilt_ids = IndexSet::new();
        for (i, (middleware_id, next_state_types)) in
            middleware_id2next_field_types.iter().enumerate()
        {
            let next_state_bindings = next_state_types
                .iter()
                .enumerate()
                .map(|(i, type_)| {
                    // TODO: naming can be improved here.
                    (format!("s_{i}"), type_.to_owned())
                })
                .collect::<BTreeMap<_, _>>();
            let next_state_type = PathType {
                package_id: PackageId::new(GENERATED_APP_PACKAGE_ID),
                rustdoc_id: None,
                base_type: vec!["crate".into(), module_name.clone(), format!("Next{i}")],
                generic_arguments: vec![],
            };

            // We register a constructor, in order to make it possible to build an instance of
            // `next_type`.
            let next_state_constructor = Callable {
                is_async: false,
                takes_self_as_ref: false,
                path: next_state_type.resolved_path(),
                output: Some(next_state_type.clone().into()),
                inputs: next_state_bindings.values().cloned().collect(),
                invocation_style: InvocationStyle::StructLiteral {
                    field_names: next_state_bindings.clone(),
                    // TODO: remove when TAIT stabilises
                    extra_field2default_value: {
                        let next_fn_name = if i + 1 < middleware_call_graphs.len() {
                            format!("middleware_{}", i + 1)
                        } else {
                            "handler".to_string()
                        };
                        BTreeMap::from([("next".into(), next_fn_name)])
                    },
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
            let HydratedComponent::WrappingMiddleware(mw) =
                component_db.hydrated_component(*middleware_id, computation_db)
            else {
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

            let HydratedComponent::WrappingMiddleware(bound_mw) =
                component_db.hydrated_component(bound_middleware_id, computation_db)
            else {
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
            let middleware_call_graph = request_scoped_ordered_call_graph(
                bound_middleware_id,
                &request_scoped_prebuilt_ids,
                &error_observer_ids,
                computation_db,
                component_db,
                constructible_db,
                package_graph,
                krate_collection,
                diagnostics,
            )?;
            // Add all request-scoped components initialised by this middleware to the set.
            extract_request_scoped_compute_nodes(
                &middleware_call_graph.call_graph,
                component_db,
                &mut request_scoped_prebuilt_ids,
            );
            middleware_id2stage_data.insert(
                *middleware_id,
                MiddlewareData {
                    call_graph: middleware_call_graph,
                    next_state: NextState {
                        type_: next_state_type,
                        field_bindings: next_state_bindings,
                    },
                },
            );
        }

        Ok(Self {
            module_name,
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

    /// Print a representation of the pipeline in graphviz's .DOT format, geared towards
    /// debugging.
    #[allow(unused)]
    pub(crate) fn print_debug_dot(
        &self,
        component_db: &ComponentDb,
        computation_db: &ComputationDb,
    ) {
        for graph in self.graph_iter() {
            graph.print_debug_dot(component_db, computation_db)
        }
    }
}

/// Extract the set of request-scoped and singleton components that are used as inputs
/// in the provided call graph.
///
/// The extracted component ids are inserted into the provided buffer set.
fn extract_long_lived_inputs(
    call_graph: &RawCallGraph,
    component_db: &ComponentDb,
    buffer: &mut IndexSet<ResolvedType>,
) {
    for node in call_graph.node_weights() {
        let CallGraphNode::InputParameter { type_, source } = node else {
            continue;
        };
        let InputParameterSource::Component(component_id) = source else {
            continue;
        };
        assert_ne!(
            component_db.lifecycle(*component_id),
            Some(&Lifecycle::Transient),
            "Transient components should not appear as inputs in a call graph"
        );
        buffer.insert(type_.to_owned());
    }
}

fn extract_request_scoped_compute_nodes(
    call_graph: &RawCallGraph,
    component_db: &ComponentDb,
    buffer: &mut IndexSet<ComponentId>,
) {
    for node in call_graph.node_weights() {
        let CallGraphNode::Compute { component_id, .. } = node else {
            continue;
        };
        if component_db.lifecycle(*component_id) == Some(&Lifecycle::RequestScoped) {
            buffer.insert(*component_id);
        }
    }
}
