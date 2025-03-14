use ahash::{HashMap, HashMapExt, HashSet, HashSetExt};
use bimap::BiHashMap;
use guppy::PackageId;
use indexmap::IndexSet;
use petgraph::Direction;
use petgraph::algo::has_path_connecting;
use petgraph::prelude::{StableDiGraph, StableGraph};
use petgraph::stable_graph::NodeIndex;

use pavex_bp_schema::Lifecycle;

use crate::compiler::analyses::components::{ComponentDb, ComponentId};
use crate::compiler::analyses::components::{
    ConsumptionMode, HydratedComponent, InsertTransformer,
};
use crate::compiler::analyses::computations::ComputationDb;
use crate::compiler::analyses::constructibles::ConstructibleDb;
use crate::compiler::analyses::user_components::ScopeId;
use crate::compiler::computation::{Computation, MatchResultVariant};
use crate::language::{Lifetime, ResolvedType, TypeReference};

use super::dependency_graph::DependencyGraph;

/// We want to code-generate a wrapping function for a given callable (the "root"), its **dependency closure**.
/// The dependency closure, leveraging the registered constructors, should either require no input
/// of its own or ask for "upstream" inputs (i.e. types that are recursive dependencies of the input
/// types for the callable that we want to invoke).
///
/// We represent the dependency closure as a directed acyclic graph (DAG), where each node is a
/// constructor (or an upstream input) and each edge represents a dependency between two constructors.
///
/// In the call graph, each constructor appears as many times as it needs to be invoked. A separate
/// node type is used for types that we can't build, the ones that the callable closure will
/// take as inputs.
///
/// # Example: request handling
///
/// Singletons should be constructed once and re-used throughout the entire lifetime of the
/// application; this implies that the generated code for handling a single request should not
/// call the singleton constructor—it should fetch it from the server state!
/// Request-scoped types, instead, should be built by the request handler closure **at most once**.
/// Transient types can be built multiple times within the lifecycle of each incoming request.
#[derive(Debug)]
pub(crate) struct CallGraph {
    /// The actual graph data structure holding nodes and edges.
    /// See [`RawCallGraph`] for more details.
    pub(crate) call_graph: RawCallGraph,
    pub(crate) root_node_index: NodeIndex,
    /// The [`ScopeId`] of the root callable.
    /// Components registered against that [`ScopeId`] should only be visible to this call graph.
    pub(crate) root_scope_id: ScopeId,
}

/// Build a [`CallGraph`] rooted in the `root_id` component.
/// The caller needs to provide the required look-up maps and a function that determines how
/// many times a callable can be invoked given its [`Lifecycle`].
/// All the graph-traversing machinery is taken care of.
pub(super) fn build_call_graph<F>(
    root_id: ComponentId,
    // The set of long-lived components that have already been initialised and should be
    // taken as inputs rather than built again.
    prebuilt_ids: &IndexSet<ComponentId>,
    error_observer_ids: &[ComponentId],
    computation_db: &ComputationDb,
    component_db: &ComponentDb,
    constructible_db: &ConstructibleDb,
    lifecycle2n_allowed_invocations: F,
    diagnostics: &mut crate::diagnostic::DiagnosticSink,
) -> Result<CallGraph, ()>
where
    F: Fn(Lifecycle) -> Option<NumberOfAllowedInvocations> + Clone,
{
    // If the dependency graph is not acyclic, we can't build a call graph—we'd get stuck in an infinite loop.
    if DependencyGraph::build(
        root_id,
        computation_db,
        component_db,
        constructible_db,
        error_observer_ids,
        lifecycle2n_allowed_invocations.clone(),
    )
    .assert_acyclic(component_db, computation_db, diagnostics)
    .is_err()
    {
        return Err(());
    }

    let root_scope_id = component_db.scope_id(root_id);
    let mut call_graph = RawCallGraph::new();

    let component_id2invocations = |component_id: ComponentId| {
        lifecycle2n_allowed_invocations(component_db.lifecycle(component_id))
    };
    let component_id2node = |id: ComponentId| {
        if let Computation::PrebuiltType(i) = component_db
            .hydrated_component(id, computation_db)
            .computation()
        {
            return CallGraphNode::InputParameter {
                type_: i.into_owned(),
                source: InputParameterSource::Component(id),
            };
        }
        if prebuilt_ids.contains(&id) {
            let resolved_component = component_db.hydrated_component(id, computation_db);
            assert!(
                !matches!(resolved_component, HydratedComponent::ErrorObserver(_)),
                "Error observers should never be prebuilt."
            );
            return CallGraphNode::InputParameter {
                type_: resolved_component.output_type().unwrap().to_owned(),
                source: InputParameterSource::Component(id),
            };
        }
        match component_id2invocations(id) {
            None => {
                let resolved_component = component_db.hydrated_component(id, computation_db);
                assert!(
                    !matches!(resolved_component, HydratedComponent::ErrorObserver(_)),
                    "Error observers should never be input parameters."
                );
                CallGraphNode::InputParameter {
                    type_: resolved_component.output_type().unwrap().to_owned(),
                    source: InputParameterSource::Component(id),
                }
            }
            Some(n_allowed_invocations) => CallGraphNode::Compute {
                component_id: id,
                n_allowed_invocations,
            },
        }
    };

    let mut transformed_node_indexes = HashSet::new();
    // We need to keep track of the nodes to which we have already attached error observers
    // (or those that we determined don't need them).
    let mut attached_observer_indexes = HashSet::new();

    // If the constructor for a type can be invoked at most once, then it should appear
    // at most once in the call graph.
    // Deduplicator makes sure of that.
    let mut node_deduplicator = NodeDeduplicator::new();
    let add_node_for_component = |graph: &mut RawCallGraph,
                                  node_deduplicator: &mut NodeDeduplicator,
                                  component_id: ComponentId| {
        let node = component_id2node(component_id);
        use {CallGraphNode::*, NumberOfAllowedInvocations::*};
        match node {
            Compute {
                n_allowed_invocations: One,
                ..
            }
            | InputParameter { .. } => node_deduplicator.add_node_at_most_once(graph, node),
            Compute {
                n_allowed_invocations: Multiple,
                ..
            } => graph.add_node(node),
            MatchBranching => unreachable!(),
        }
    };

    let root_node_index = add_node_for_component(&mut call_graph, &mut node_deduplicator, root_id);
    let mut nodes_to_be_visited: IndexSet<VisitorStackElement> =
        IndexSet::from_iter([VisitorStackElement::orphan(root_node_index)]);
    let mut processed_nodes = HashSet::new();

    loop {
        while let Some(node_to_be_visited) = nodes_to_be_visited.pop() {
            let (current_index, neighbour_index) =
                (node_to_be_visited.node_index, node_to_be_visited.neighbour);

            if let Some(neighbour_index) = neighbour_index {
                match neighbour_index {
                    VisitorNeighbour::Parent(parent_index, edge_metadata) => {
                        call_graph.update_edge(parent_index, current_index, edge_metadata);
                    }
                    VisitorNeighbour::Child(child_index, edge_metadata) => {
                        call_graph.update_edge(current_index, child_index, edge_metadata);
                    }
                }
            }

            // We've already processed dependencies for this node.
            if processed_nodes.contains(&current_index) {
                continue;
            }

            // We need to recursively build the input types for all our compute components;
            if let CallGraphNode::Compute { component_id, .. } = call_graph[current_index].clone() {
                let component = component_db.hydrated_component(component_id, computation_db);
                let input_types = match component {
                    HydratedComponent::Constructor(constructor) => {
                        constructor.input_types().to_vec()
                    }
                    HydratedComponent::ConfigType(..) | HydratedComponent::PrebuiltType(..) => {
                        vec![]
                    }
                    HydratedComponent::RequestHandler(r) => r.input_types().to_vec(),
                    HydratedComponent::PostProcessingMiddleware(pp) => pp.input_types().to_vec(),
                    HydratedComponent::PreProcessingMiddleware(pp) => pp.input_types().to_vec(),
                    HydratedComponent::Transformer(c, info) => {
                        let mut inputs = c.input_types().to_vec();
                        // The component we are transforming must have been added to the graph
                        // before the transformer.
                        inputs.remove(info.input_index);
                        inputs
                    }
                    HydratedComponent::WrappingMiddleware(mw) => {
                        let mut input_types = mw.input_types().to_vec();
                        let next_type = &input_types[mw.next_input_index()];
                        if !next_type.unassigned_generic_type_parameters().is_empty() {
                            // If we haven't assigned a concrete type to the `Next` type parameter,
                            // we have no idea what its input types are going to be, therefore
                            // we skip it for now.
                            input_types.remove(mw.next_input_index());
                        }
                        input_types
                    }
                    HydratedComponent::ErrorObserver(eo) => {
                        let mut inputs: Vec<_> = eo.input_types().to_vec();
                        inputs.remove(eo.error_input_index);
                        inputs
                    }
                };
                for input_type in input_types {
                    if let Some((constructor_id, consumption_mode)) =
                        constructible_db.get(root_scope_id, &input_type, component_db.scope_graph())
                    {
                        nodes_to_be_visited.insert(VisitorStackElement {
                            node_index: add_node_for_component(
                                &mut call_graph,
                                &mut node_deduplicator,
                                constructor_id,
                            ),
                            neighbour: Some(VisitorNeighbour::Child(
                                current_index,
                                consumption_mode.into(),
                            )),
                        });
                    } else {
                        let index = node_deduplicator.add_node_at_most_once(
                            &mut call_graph,
                            CallGraphNode::InputParameter {
                                type_: input_type,
                                source: InputParameterSource::External,
                            },
                        );
                        call_graph.update_edge(index, current_index, CallGraphEdgeMetadata::Move);
                    }
                }
            }

            processed_nodes.insert(current_index);
        }

        let indexes = call_graph.node_indices().collect::<Vec<_>>();
        // We might need to go through multiple cycles of applying transformers
        // until the graph stabilizes (i.e. we reach a fixed point).
        let graph_size_before_transformations = indexes.len();

        // For each node, we add the respective transformers, if they have been registered.
        for node_index in indexes {
            if transformed_node_indexes.contains(&node_index) {
                continue;
            }
            'inner: {
                let node = call_graph[node_index].clone();
                let CallGraphNode::Compute { component_id, .. } = node else {
                    transformed_node_indexes.insert(node_index);
                    break 'inner;
                };
                let Some(transformer_ids) = component_db.transformer_ids(component_id) else {
                    transformed_node_indexes.insert(node_index);
                    break 'inner;
                };
                for transformer_id in transformer_ids {
                    let HydratedComponent::Transformer(_, transformer_info) =
                        component_db.hydrated_component(*transformer_id, computation_db)
                    else {
                        unreachable!()
                    };
                    if transformer_info.when_to_insert == InsertTransformer::Lazily {
                        continue;
                    }
                    // Not all transformers might be relevant to this `CallGraph`, we need to take their scope into account.
                    let transformer_scope_id = component_db.scope_id(*transformer_id);
                    if root_scope_id
                        .is_descendant_of(transformer_scope_id, component_db.scope_graph())
                    {
                        nodes_to_be_visited.insert(VisitorStackElement {
                            node_index: add_node_for_component(
                                &mut call_graph,
                                &mut node_deduplicator,
                                *transformer_id,
                            ),
                            neighbour: Some(VisitorNeighbour::Parent(
                                node_index,
                                transformer_info.transformation_mode.into(),
                            )),
                        });
                    }
                }
            }
            transformed_node_indexes.insert(node_index);
        }

        'observers: {
            if error_observer_ids.is_empty() {
                break 'observers;
            }
            let indexes = call_graph.node_indices().collect::<Vec<_>>();
            for node_index in indexes {
                if attached_observer_indexes.contains(&node_index) {
                    continue;
                }
                'inner: {
                    let node = call_graph[node_index].clone();
                    let CallGraphNode::Compute { component_id, .. } = node else {
                        attached_observer_indexes.insert(node_index);
                        break 'inner;
                    };
                    if !component_db.is_error_handler(component_id) {
                        attached_observer_indexes.insert(node_index);
                        break 'inner;
                    };

                    // Error handlers always have a single child node: an `IntoResponse` transformer.
                    // We want to insert our error observers in between the error handler and its child.
                    #[cfg(debug_assertions)]
                    {
                        let child_node_indexes = call_graph
                            .neighbors_directed(node_index, Direction::Outgoing)
                            .collect::<Vec<_>>();
                        assert!(child_node_indexes.len() <= 1);
                    }
                    let Some(child_id) = call_graph
                        .neighbors_directed(node_index, Direction::Outgoing)
                        .next()
                    else {
                        // The transformer might not have been processed yet, we'll come back to this
                        // node later.
                        break 'inner;
                    };

                    // There are two topologies for error handlers:
                    // - Directly downstream of the `Err` branch of match, which in turn has `pavex::Error::new` as a child
                    // - Directly downstream of a `pavex::Error::new` invocations
                    // We want to find the `pavex::Error::new` node index for this error.
                    let pavex_error_new_node_index = match call_graph
                        .neighbors_directed(node_index, Direction::Incoming)
                        .find(|parent_index| {
                            let parent_node = &call_graph[*parent_index];
                            let CallGraphNode::Compute { component_id, .. } = parent_node else {
                                return false;
                            };
                            let computation = component_db
                                .hydrated_component(*component_id, computation_db)
                                .computation();
                            let Computation::MatchResult(m) = computation else {
                                return false;
                            };
                            m.variant == MatchResultVariant::Err
                        }) {
                        None => call_graph
                            .neighbors_directed(node_index, Direction::Incoming)
                            .find_map(|parent_index| {
                                let parent_node = &call_graph[parent_index];
                                let CallGraphNode::Compute { component_id, .. } = parent_node
                                else {
                                    return None;
                                };
                                let component =
                                    component_db.hydrated_component(*component_id, computation_db);
                                if component.output_type()? == &component_db.pavex_error {
                                    Some(parent_index)
                                } else {
                                    None
                                }
                            })
                            .unwrap(),
                        Some(match_error_node_index) => {
                            // We can now find the `pavex::Error::new` invocation as one of
                            // the children of the error matcher.
                            let Some(pavex_error_new_node_index) = call_graph
                                .neighbors_directed(match_error_node_index, Direction::Outgoing)
                                .find(|&child_index| child_index != node_index)
                            else {
                                break 'inner;
                            };
                            pavex_error_new_node_index
                        }
                    };

                    let mut previous_index = None;
                    for error_observer_id in error_observer_ids {
                        let error_observer_node_index = add_node_for_component(
                            &mut call_graph,
                            &mut node_deduplicator,
                            *error_observer_id,
                        );
                        if let Some(previous_index) = previous_index {
                            call_graph.update_edge(
                                previous_index,
                                error_observer_node_index,
                                CallGraphEdgeMetadata::HappensBefore,
                            );
                        }
                        call_graph.update_edge(
                            pavex_error_new_node_index,
                            error_observer_node_index,
                            CallGraphEdgeMetadata::SharedBorrow,
                        );
                        nodes_to_be_visited.insert(VisitorStackElement {
                            node_index: error_observer_node_index,
                            neighbour: None,
                        });
                        previous_index = Some(error_observer_node_index);
                    }
                    if let Some(previous_index) = previous_index {
                        call_graph.update_edge(
                            previous_index,
                            child_id,
                            CallGraphEdgeMetadata::HappensBefore,
                        );
                    }

                    attached_observer_indexes.insert(node_index);
                }
            }
        }

        if nodes_to_be_visited.is_empty()
            && call_graph.node_count() == graph_size_before_transformations
        {
            break;
        }
    }

    // We now perform further graph transformations that are not related to dependency injection
    // and whose results do not require further iterations of the fixed-point algorithm we just
    // went through.
    inject_match_branching_nodes(computation_db, component_db, &mut call_graph);
    take_references_as_inputs_if_they_suffice(&mut call_graph);

    // `root_node_index` might point to a `Compute` node that returns a `Result`, therefore
    // it might no longer be without descendants after our insertion of `MatchBranching` nodes.
    // If that's the case, we determine a new `root_node_index` by picking the terminal node
    // that descends from `root_node_index` on the `Ok`-matcher side.
    let root_node = &call_graph[root_node_index];
    let root_component = root_node
        .as_hydrated_component(component_db, computation_db)
        .unwrap();
    let root_is_fallible = root_component.output_type().unwrap().is_result();
    let search_start_index = if root_is_fallible {
        let match_node_index = {
            let children: Vec<_> = call_graph
                .neighbors_directed(root_node_index, Direction::Outgoing)
                .collect();
            assert_eq!(children.len(), 1);
            children[0]
        };
        let ok_matcher_node_index = {
            let children: Vec<_> = call_graph
                .neighbors_directed(match_node_index, Direction::Outgoing)
                .collect();
            assert_eq!(children.len(), 2);
            children
                .into_iter()
                .find(|node_ix| {
                    let node = &call_graph[*node_ix];
                    if let CallGraphNode::Compute { component_id, .. } = node {
                        if let HydratedComponent::Transformer(Computation::MatchResult(m), ..) =
                            component_db.hydrated_component(*component_id, computation_db)
                        {
                            if m.variant == MatchResultVariant::Ok {
                                return true;
                            }
                        }
                    }
                    false
                })
                .unwrap()
        };
        ok_matcher_node_index
    } else {
        root_node_index
    };
    let new_root_index = {
        let downstream_sources: Vec<_> = call_graph
            .externals(Direction::Outgoing)
            .filter(|index| has_path_connecting(&call_graph, search_start_index, *index, None))
            .collect();
        assert_eq!(downstream_sources.len(), 1);
        downstream_sources[0]
    };
    enforce_invariants(
        &call_graph,
        error_observer_ids.len(),
        component_db,
        computation_db,
    );

    let call_graph = CallGraph {
        call_graph,
        root_node_index: new_root_index,
        root_scope_id,
    };
    Ok(call_graph)
}

fn enforce_invariants(
    call_graph: &RawCallGraph,
    n_unique_error_observers: usize,
    component_db: &ComponentDb,
    computation_db: &ComputationDb,
) {
    let mut n_error_observers = 0;
    let mut n_errors = 0;
    for node in call_graph.node_weights() {
        if let CallGraphNode::MatchBranching { .. } = node {
            n_errors += 1;
            continue;
        }
        if let Some(HydratedComponent::ErrorObserver(_)) =
            node.as_hydrated_component(component_db, computation_db)
        {
            n_error_observers += 1;
        };
    }
    let expected_error_observers = n_errors * n_unique_error_observers;
    assert_eq!(
        expected_error_observers, n_error_observers,
        "There should be {expected_error_observers} error observers in the graph, but we found {n_error_observers}."
    )
}

impl CallGraph {
    /// Print a representation of the [`CallGraph`] in graphviz's .DOT format, geared towards
    /// debugging.
    #[allow(unused)]
    pub(crate) fn print_debug_dot(
        &self,
        component_db: &ComponentDb,
        computation_db: &ComputationDb,
    ) {
        eprintln!("{}", self.debug_dot(component_db, computation_db));
    }

    /// Return a representation of the [`CallGraph`] in graphviz's .DOT format, geared towards
    /// debugging.
    #[allow(unused)]
    pub(crate) fn debug_dot(
        &self,
        component_db: &ComponentDb,
        computation_db: &ComputationDb,
    ) -> String {
        let config = [
            petgraph::dot::Config::EdgeNoLabel,
            petgraph::dot::Config::NodeNoLabel,
        ];
        format!(
            "{:?}",
            petgraph::dot::Dot::with_attr_getters(
                &self.call_graph,
                &config,
                &|_, edge| match edge.weight() {
                    CallGraphEdgeMetadata::Move => "".to_string(),
                    CallGraphEdgeMetadata::SharedBorrow => "label = \"&\"".to_string(),
                    CallGraphEdgeMetadata::ExclusiveBorrow => "label = \"&mut \"".to_string(),
                    CallGraphEdgeMetadata::HappensBefore => "label = \"before\"".to_string(),
                },
                &|_, (id, node)| {
                    match node {
                        CallGraphNode::Compute { component_id, .. } => {
                            match component_db
                                .hydrated_component(*component_id, computation_db)
                                .computation()
                            {
                                Computation::MatchResult(m) => {
                                    format!("label = \"{:?} -> {:?}\"", m.input, m.output)
                                }
                                Computation::Callable(c) => {
                                    format!("label = \"{c:?}\"")
                                }
                                Computation::PrebuiltType(i) => {
                                    format!("label = \"{i:?}\"")
                                }
                            }
                        }
                        CallGraphNode::InputParameter { type_, .. } => {
                            format!("label = \"{type_:?}\"")
                        }
                        CallGraphNode::MatchBranching => "label = \"`match`\"".to_string(),
                    }
                },
            )
        )
    }
}

/// We traverse the graph looking for fallible compute nodes.
/// For each of them we add a `MatchBranching` node, in between the ancestor `Compute` node
/// for a `Result` type and the corresponding descendants `MatchResult` nodes.
///
/// In other words: we want to go from
///
/// ```text
/// Compute(Result) -> MatchResult(Ok)
///                    MatchResult(Err)
/// ```
///
/// to
///
/// ```text
/// Compute(Result) -> MatchBranching -> MatchResult(Ok)
///                                   -> MatchResult(Err)
/// ```
fn inject_match_branching_nodes(
    computation_db: &ComputationDb,
    component_db: &ComponentDb,
    call_graph: &mut StableGraph<CallGraphNode, CallGraphEdgeMetadata>,
) {
    let indexes = call_graph.node_indices().collect::<Vec<_>>();
    for node_index in indexes {
        let node = call_graph[node_index].clone();
        let CallGraphNode::Compute { component_id, .. } = node else {
            continue;
        };
        let Some((ok_match_id, err_match_id)) = component_db.match_ids(component_id) else {
            // We only want to look at fallible components
            continue;
        };
        let child_node_indexes = call_graph
            .neighbors_directed(node_index, Direction::Outgoing)
            .collect::<Vec<_>>();
        assert_eq!(
            child_node_indexes.len(),
            2,
            "Fallible nodes should have two child nodes. This is not the case for node {:?}.\nGraph:\n{}",
            node_index,
            call_graph.debug_dot(component_db, computation_db)
        );

        let first_child_node_index = child_node_indexes[0];
        let second_child_node_index = child_node_indexes[1];
        for idx in [first_child_node_index, second_child_node_index] {
            match &call_graph[idx] {
                CallGraphNode::Compute {
                    component_id: child_id,
                    ..
                } => {
                    assert!(
                        [ok_match_id, err_match_id].contains(&child_id),
                        "{child_id:?} is neither the Ok-matcher ({ok_match_id:?}) nor the Err-matcher ({err_match_id:?}) for fallible component `{component_id:?}`.\n\
                        {child_id:?}: {:?}\n\
                        {component_id:?}: {:?}",
                        component_db.hydrated_component(*child_id, computation_db),
                        component_db.hydrated_component(component_id, computation_db),
                    );
                }
                _ => unreachable!(),
            }
        }

        let branching_node_index = call_graph.add_node(CallGraphNode::MatchBranching);
        let first_edge = call_graph
            .find_edge(node_index, first_child_node_index)
            .unwrap();
        let second_edge = call_graph
            .find_edge(node_index, second_child_node_index)
            .unwrap();
        call_graph.remove_edge(first_edge).unwrap();
        call_graph.remove_edge(second_edge).unwrap();
        call_graph.add_edge(
            branching_node_index,
            first_child_node_index,
            CallGraphEdgeMetadata::Move,
        );
        call_graph.add_edge(
            branching_node_index,
            second_child_node_index,
            CallGraphEdgeMetadata::Move,
        );
        call_graph.add_edge(
            node_index,
            branching_node_index,
            CallGraphEdgeMetadata::Move,
        );
    }
}

/// If we only need to borrow an input parameter, we refactor the graph to use a `Move` edge
/// from a reference input type rather than a `SharedBorrow`/`ExclusiveBorrow`
/// edge from a non-reference input type.
///
/// This is done in order to generate a closure that asks for a reference instead of an owned
/// type, therefore avoiding an unnecessary clone for the caller.
fn take_references_as_inputs_if_they_suffice(
    call_graph: &mut StableGraph<CallGraphNode, CallGraphEdgeMetadata>,
) {
    let indexes = call_graph
        .externals(Direction::Incoming)
        .collect::<Vec<_>>();
    for node_index in indexes {
        let node = &call_graph[node_index];
        let CallGraphNode::InputParameter {
            source,
            type_: input_type,
        } = node
        else {
            continue;
        };
        match input_type {
            ResolvedType::Reference(_) => continue,
            _ => {
                let mut by_value = false;
                let mut borrowed_mutably = false;
                for edge in call_graph.edges_directed(node_index, Direction::Outgoing) {
                    match edge.weight() {
                        CallGraphEdgeMetadata::Move => by_value = true,
                        CallGraphEdgeMetadata::SharedBorrow => {}
                        CallGraphEdgeMetadata::ExclusiveBorrow => borrowed_mutably = true,
                        CallGraphEdgeMetadata::HappensBefore => {}
                    }
                }
                if !by_value {
                    let reference_input_type = ResolvedType::Reference(TypeReference {
                        is_mutable: borrowed_mutably,
                        lifetime: Lifetime::Elided,
                        inner: Box::new(input_type.to_owned()),
                    });
                    let reference_input_node_index =
                        call_graph.add_node(CallGraphNode::InputParameter {
                            source: *source,
                            type_: reference_input_type,
                        });
                    for neighbour_index in call_graph
                        .neighbors_directed(node_index, Direction::Outgoing)
                        .collect::<Vec<_>>()
                    {
                        call_graph.update_edge(
                            reference_input_node_index,
                            neighbour_index,
                            CallGraphEdgeMetadata::Move,
                        );
                    }

                    call_graph.remove_node(node_index);
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq)]
/// The edges in the call graph represent the dependency relationships between the nodes.
/// All edges are **directed**.
///
/// If the edge is either `Move` or `SharedBorrow`, the edge goes from the node that provides the input
/// to the node that requires it.
/// The type of edge determines *how* the input is consumed.
///
/// If the edge is `HappensBefore`, it encodes a temporal relationship but there is no
/// output-input relationship between the two connected nodes.
pub(crate) enum CallGraphEdgeMetadata {
    /// The output of the source node is consumed by value—e.g. `String` in `fn handler(input: String)`.
    Move,
    /// The target node requires a shared reference to the output type of the source node —e.g. `&MyStruct` in
    /// `fn handler(input: &MyStruct)`.
    SharedBorrow,
    /// The target node requires a mutable reference to the output type of the source node —e.g. `&mut MyStruct`
    /// in `fn handler(input: &mut MyStruct)`.
    ExclusiveBorrow,
    /// The computation in the source node must be invoked before the computation in the target node.
    HappensBefore,
}

impl From<ConsumptionMode> for CallGraphEdgeMetadata {
    fn from(value: ConsumptionMode) -> Self {
        match value {
            ConsumptionMode::Move => CallGraphEdgeMetadata::Move,
            ConsumptionMode::SharedBorrow => CallGraphEdgeMetadata::SharedBorrow,
            ConsumptionMode::ExclusiveBorrow => CallGraphEdgeMetadata::ExclusiveBorrow,
        }
    }
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub(crate) enum CallGraphNode {
    Compute {
        component_id: ComponentId,
        n_allowed_invocations: NumberOfAllowedInvocations,
    },
    MatchBranching,
    InputParameter {
        /// Where we expect the input value to be sourced from.
        source: InputParameterSource,
        /// The type that will be taken as an input parameter by the generated dependency closure.
        type_: ResolvedType,
    },
}

#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq)]
/// The expected provider of the required input parameter.
pub(crate) enum InputParameterSource {
    /// It will be built by invoking a component.
    Component(ComponentId),
    /// It will be provided by the user of the generated code.
    External,
}

impl CallGraphNode {
    /// If this node is a `Compute` node, return the [`HydratedComponent`] associated with its
    /// [`ComponentId`].
    /// If not, return `None`.
    #[allow(unused)]
    pub fn as_hydrated_component<'a>(
        &self,
        component_db: &'a ComponentDb,
        computation_db: &'a ComputationDb,
    ) -> Option<HydratedComponent<'a>> {
        let CallGraphNode::Compute { component_id, .. } = self else {
            return None;
        };
        Some(component_db.hydrated_component(*component_id, computation_db))
    }

    /// Return the [`ComponentId`] associated with this node, if any.
    pub fn component_id(&self) -> Option<ComponentId> {
        match self {
            CallGraphNode::Compute { component_id, .. } => Some(*component_id),
            CallGraphNode::MatchBranching => None,
            CallGraphNode::InputParameter { source, .. } => match source {
                InputParameterSource::Component(id) => Some(*id),
                InputParameterSource::External => None,
            },
        }
    }
}

#[derive(Clone, Debug, Copy, Hash, Eq, PartialEq)]
/// How many times can a certain constructor be invoked within the body of
/// the code-generated function?
pub(crate) enum NumberOfAllowedInvocations {
    /// At most once.
    One,
    /// As many times as you want to.
    Multiple,
}

#[derive(Debug, Eq, PartialEq, Hash)]
struct VisitorStackElement {
    node_index: NodeIndex,
    neighbour: Option<VisitorNeighbour>,
}

#[derive(Debug, Eq, PartialEq, Hash)]
enum VisitorNeighbour {
    Parent(NodeIndex, CallGraphEdgeMetadata),
    Child(NodeIndex, CallGraphEdgeMetadata),
}

impl VisitorStackElement {
    /// A short-cut to add a node without a parent to the visitor stack.
    fn orphan(node_index: NodeIndex) -> Self {
        Self {
            node_index,
            neighbour: None,
        }
    }
}

struct NodeDeduplicator(HashMap<CallGraphNode, NodeIndex>);

impl NodeDeduplicator {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn add_node_at_most_once(
        &mut self,
        graph: &mut RawCallGraph,
        node: CallGraphNode,
    ) -> NodeIndex {
        assert!(!matches!(node, CallGraphNode::MatchBranching { .. }));
        self.0.get(&node).cloned().unwrap_or_else(|| {
            let index = graph.add_node(node.clone());
            self.0.insert(node, index);
            index
        })
    }
}

/// The graph data structured that [`CallGraph`] is built on.
///
/// We use a [`StableDiGraph`] to represent the call graph because:
/// - we want indices for existing nodes to be unaffected (i.e. "stable") when nodes are removed
/// - node relationships are directed, e.g. `A -> B` means that `B` requires the output of `A` as an input.
pub(crate) type RawCallGraph = StableDiGraph<CallGraphNode, CallGraphEdgeMetadata>;

/// Methods for [`RawCallGraph`].
///
/// We use an extension trait since [`RawCallGraph`] is a type alias that points to a foreign type.
#[allow(unused)]
pub(crate) trait RawCallGraphExt {
    /// Return the set of types that must be provided as input to (recursively) build the handler's
    /// input parameters and invoke it.
    ///
    /// We return a `IndexSet` instead of a `HashSet` because we want a consistent ordering for the input
    /// parameters—it will be used in other parts of the crate to provide instances of those types
    /// in the expected order.
    fn required_input_types(&self) -> IndexSet<ResolvedType>;
    /// Return a representation of the [`CallGraph`] in graphviz's .DOT format.
    fn dot(
        &self,
        package_ids2names: &BiHashMap<PackageId, String>,
        component_db: &ComponentDb,
        computation_db: &ComputationDb,
    ) -> String;
    /// Print a representation of the [`CallGraph`] in graphviz's .DOT format, geared towards
    /// debugging.
    fn print_debug_dot(&self, component_db: &ComponentDb, computation_db: &ComputationDb);
    /// Return a representation of the [`CallGraph`] in graphviz's .DOT format, geared towards
    /// debugging.
    fn debug_dot(&self, component_db: &ComponentDb, computation_db: &ComputationDb) -> String;
}

impl RawCallGraphExt for RawCallGraph {
    fn required_input_types(&self) -> IndexSet<ResolvedType> {
        self.node_weights()
            .filter_map(|node| match node {
                CallGraphNode::Compute { .. } | CallGraphNode::MatchBranching => None,
                CallGraphNode::InputParameter { type_, .. } => Some(type_),
            })
            .cloned()
            .collect()
    }

    fn dot(
        &self,
        package_ids2names: &BiHashMap<PackageId, String>,
        component_db: &ComponentDb,
        computation_db: &ComputationDb,
    ) -> String {
        let config = [
            petgraph::dot::Config::EdgeNoLabel,
            petgraph::dot::Config::NodeNoLabel,
        ];
        format!(
            "{:?}",
            petgraph::dot::Dot::with_attr_getters(
                self,
                &config,
                &|_, edge| match edge.weight() {
                    CallGraphEdgeMetadata::Move => "".to_string(),
                    CallGraphEdgeMetadata::SharedBorrow => "label = \"&\"".to_string(),
                    CallGraphEdgeMetadata::ExclusiveBorrow => "label = \"&mut \"".to_string(),
                    CallGraphEdgeMetadata::HappensBefore =>
                        "label = \"happens before\"".to_string(),
                },
                &|_, (_, node)| {
                    match node {
                        CallGraphNode::Compute { component_id, .. } => match component_db
                            .hydrated_component(*component_id, computation_db)
                            .computation()
                        {
                            Computation::Callable(c) => {
                                format!("label = \"{}\"", c.render_signature(package_ids2names))
                            }
                            Computation::MatchResult(m) => {
                                format!(
                                    "label = \"{} -> {}\"",
                                    m.input.render_type(package_ids2names),
                                    m.output.render_type(package_ids2names)
                                )
                            }
                            Computation::PrebuiltType(i) => {
                                format!("label = \"{}\"", i.render_type(package_ids2names))
                            }
                        },
                        CallGraphNode::InputParameter { type_, .. } => {
                            format!("label = \"{}\"", type_.render_type(package_ids2names))
                        }
                        CallGraphNode::MatchBranching => "label = \"`match`\"".to_string(),
                    }
                },
            )
        )
    }

    #[allow(unused)]
    fn print_debug_dot(&self, component_db: &ComponentDb, computation_db: &ComputationDb) {
        eprintln!("{}", self.debug_dot(component_db, computation_db));
    }

    #[allow(unused)]
    fn debug_dot(&self, component_db: &ComponentDb, computation_db: &ComputationDb) -> String {
        let config = [
            petgraph::dot::Config::EdgeNoLabel,
            petgraph::dot::Config::NodeNoLabel,
        ];
        format!(
            "{:?}",
            petgraph::dot::Dot::with_attr_getters(
                self,
                &config,
                &|_, edge| match edge.weight() {
                    CallGraphEdgeMetadata::Move => "".to_string(),
                    CallGraphEdgeMetadata::SharedBorrow => "label = \"&\"".to_string(),
                    CallGraphEdgeMetadata::ExclusiveBorrow => "label = \"&mut \"".to_string(),
                    CallGraphEdgeMetadata::HappensBefore =>
                        "label = \"happens before\"".to_string(),
                },
                &|_, (index, node)| {
                    let label = match node {
                        CallGraphNode::Compute { component_id, .. } => {
                            let component_label = match component_db
                                .hydrated_component(*component_id, computation_db)
                                .computation()
                            {
                                Computation::MatchResult(m) => {
                                    format!("{:?} -> {:?}", m.input, m.output)
                                }
                                Computation::Callable(c) => {
                                    format!("{c:?}")
                                }
                                Computation::PrebuiltType(i) => {
                                    format!("{i:?}")
                                }
                            };
                            format!(
                                "{component_label} (Component ix: {})",
                                component_id.into_raw().into_u32()
                            )
                        }
                        CallGraphNode::InputParameter { type_, .. } => {
                            format!("{type_:?}")
                        }
                        CallGraphNode::MatchBranching => "`match`".to_string(),
                    };
                    format!("label = \"{label} (Node ix: {})\"", index.index())
                },
            )
        )
    }
}
