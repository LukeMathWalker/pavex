use crate::compiler::analyses::components::{
    ComponentDb, ComponentId, ConsumptionMode, HydratedComponent,
};
use crate::compiler::analyses::computations::ComputationDb;
use crate::compiler::analyses::constructibles::ConstructibleDb;
use crate::compiler::analyses::user_components::ScopeId;
use crate::compiler::computation::{Computation, MatchResultVariant};
use ahash::{HashMap, HashMapExt, HashSet, HashSetExt};
use bimap::BiHashMap;
use guppy::PackageId;
use indexmap::IndexSet;
use pavex::blueprint::constructor::Lifecycle;
use petgraph::prelude::{StableDiGraph, StableGraph};
use petgraph::stable_graph::NodeIndex;
use petgraph::visit::Dfs;
use petgraph::Direction;

use crate::language::{ResolvedType, TypeReference};

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
    computation_db: &ComputationDb,
    component_db: &ComponentDb,
    constructible_db: &ConstructibleDb,
    lifecycle2n_allowed_invocations: F,
    diagnostics: &mut Vec<miette::Error>,
) -> Result<CallGraph, ()>
where
    F: Fn(&Lifecycle) -> Option<NumberOfAllowedInvocations> + Clone,
{
    // If the dependency graph is not acyclic, we can't build a call graph—we'd get stuck in an infinite loop.
    if DependencyGraph::build(
        root_id,
        computation_db,
        component_db,
        constructible_db,
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
        // We don't expect to invoke this function for response transformers, therefore
        // it's fine to unwrap here.
        let lifecycle = component_db.lifecycle(component_id).unwrap();
        lifecycle2n_allowed_invocations(lifecycle)
    };
    let component_id2node = |id: ComponentId| {
        if let Computation::FrameworkItem(i) = component_db
            .hydrated_component(id, computation_db)
            .computation()
        {
            CallGraphNode::InputParameter {
                type_: i.into_owned(),
                source: InputParameterSource::Component(id),
            }
        } else {
            let n_invocations = component_id2invocations(id);
            match n_invocations {
                None => {
                    let resolved_component = component_db.hydrated_component(id, computation_db);
                    CallGraphNode::InputParameter {
                        type_: resolved_component.output_type().to_owned(),
                        source: InputParameterSource::Component(id),
                    }
                }
                Some(n_allowed_invocations) => CallGraphNode::Compute {
                    component_id: id,
                    n_allowed_invocations,
                },
            }
        }
    };

    let mut transformed_node_indexes = HashSet::new();
    let mut handled_error_node_indexes = HashSet::new();

    let mut nodes_to_be_visited: IndexSet<VisitorStackElement> =
        IndexSet::from_iter([VisitorStackElement::orphan(root_id)]);

    // If the constructor for a type can be invoked at most once, then it should appear
    // at most once in the call graph. This mapping, and the corresponding Rust closure, are used
    // to make sure of that.
    let mut indexes_for_unique_nodes = HashMap::<CallGraphNode, NodeIndex>::new();
    let mut add_node_at_most_once = |graph: &mut RawCallGraph, node: CallGraphNode| {
        assert!(!matches!(node, CallGraphNode::MatchBranching { .. }));
        indexes_for_unique_nodes
            .get(&node)
            .cloned()
            .unwrap_or_else(|| {
                let index = graph.add_node(node.clone());
                indexes_for_unique_nodes.insert(node, index);
                index
            })
    };

    loop {
        while let Some(node_to_be_visited) = nodes_to_be_visited.pop() {
            let (component_id, neighbour_index) = (
                node_to_be_visited.component_id,
                node_to_be_visited.neighbour,
            );
            let current_index = {
                let call_graph_node = component_id2node(component_id);
                match call_graph_node {
                    CallGraphNode::Compute {
                        n_allowed_invocations: NumberOfAllowedInvocations::One,
                        ..
                    }
                    | CallGraphNode::InputParameter { .. } => {
                        add_node_at_most_once(&mut call_graph, call_graph_node)
                    }
                    CallGraphNode::Compute {
                        n_allowed_invocations: NumberOfAllowedInvocations::Multiple,
                        ..
                    } => call_graph.add_node(call_graph_node),
                    CallGraphNode::MatchBranching => unreachable!(),
                }
            };

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

            // We need to recursively build the input types for all our compute components;
            if let CallGraphNode::Compute { component_id, .. } = call_graph[current_index].clone() {
                let component = component_db.hydrated_component(component_id, computation_db);
                let component_scope = component_db.scope_id(component_id);
                let input_types = match component {
                    HydratedComponent::Constructor(constructor) => {
                        constructor.input_types().to_vec()
                    }
                    HydratedComponent::RequestHandler(r) => r.input_types().to_vec(),
                    HydratedComponent::ErrorHandler(error_handler) => error_handler
                        .input_types()
                        .iter()
                        // We have already added the error -> error handler edge at this stage.
                        .filter(|&t| error_handler.error_type_ref() != t)
                        .map(|t| t.to_owned())
                        .collect(),
                    HydratedComponent::Transformer(_) => {
                        // We don't allow/need dependency injection for transformers at the moment.
                        vec![]
                    }
                };
                for input_type in input_types {
                    if let Some((constructor_id, consumption_mode)) = constructible_db.get(
                        component_scope,
                        &input_type,
                        component_db.scope_graph(),
                    ) {
                        nodes_to_be_visited.insert(VisitorStackElement {
                            component_id: constructor_id,
                            neighbour: Some(VisitorNeighbour::Child(
                                current_index,
                                consumption_mode.into(),
                            )),
                        });
                    } else {
                        let index = add_node_at_most_once(
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
        }

        // For each node, we try to add a `Compute` node for the respective
        // error handler (if one was registered).
        let indexes = call_graph.node_indices().collect::<Vec<_>>();
        // We might need to go through multiple cycles of applying transformers
        // until the graph stabilizes (i.e. we reach a fixed point).
        let graph_size_before_transformations = indexes.len();

        for node_index in indexes {
            if handled_error_node_indexes.contains(&node_index) {
                continue;
            }
            'inner: {
                let node = call_graph[node_index].clone();
                let CallGraphNode::Compute {
                    component_id,
                    ..
                } = node else
                {
                    break 'inner;
                };
                if let Some(error_handler_id) = component_db.error_handler_id(component_id) {
                    nodes_to_be_visited.insert(VisitorStackElement {
                        component_id: *error_handler_id,
                        neighbour: Some(VisitorNeighbour::Parent(
                            node_index,
                            CallGraphEdgeMetadata::SharedBorrow,
                        )),
                    });
                }
            }
            handled_error_node_indexes.insert(node_index);
        }

        // For each node, we add the respective transformers, if they have been registered.
        let indexes = call_graph.node_indices().collect::<Vec<_>>();
        for node_index in indexes {
            if transformed_node_indexes.contains(&node_index) {
                continue;
            }
            'inner: {
                let node = call_graph[node_index].clone();
                let CallGraphNode::Compute {
                    component_id, n_allowed_invocations,
                } = node else {
                    break 'inner;
                };
                let Some(transformer_ids) = component_db.transformer_ids(component_id) else {
                    break 'inner;
                };
                for transformer_id in transformer_ids {
                    // Not all transformers might be relevant to this `CallGraph`, we need to take their scope into account.
                    let transformer_scope_id = component_db.scope_id(*transformer_id);
                    if root_scope_id
                        .is_descendant_of(transformer_scope_id, component_db.scope_graph())
                    {
                        let transformer_node_index = call_graph.add_node(CallGraphNode::Compute {
                            component_id: *transformer_id,
                            n_allowed_invocations,
                        });
                        // TODO: Is it correct to assume move semantics here?
                        call_graph.update_edge(
                            node_index,
                            transformer_node_index,
                            CallGraphEdgeMetadata::Move,
                        );
                    }
                }
            }
            transformed_node_indexes.insert(node_index);
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
    // If that's the case, we determine a new `root_node_index` by picking the `Ok`
    // variant that descends from `root_node_index`.
    let root_node_index = {
        // Very hacky way of getting the index of the root node
        let root_node = CallGraphNode::Compute {
            component_id: root_id,
            n_allowed_invocations: NumberOfAllowedInvocations::One,
        };
        add_node_at_most_once(&mut call_graph, root_node)
    };
    let root_node_index = if call_graph
        .neighbors_directed(root_node_index, Direction::Outgoing)
        .count()
        != 0
    {
        let mut dfs = Dfs::new(&call_graph, root_node_index);
        let mut new_root_index = root_node_index;
        while let Some(node_index) = dfs.next(&call_graph) {
            let node = &call_graph[node_index];
            if let CallGraphNode::Compute { component_id, .. } = node {
                if let HydratedComponent::Transformer(Computation::MatchResult(m)) =
                    component_db.hydrated_component(*component_id, computation_db)
                {
                    if m.variant == MatchResultVariant::Err {
                        continue;
                    }
                }
                new_root_index = node_index;
            }
        }
        new_root_index
    } else {
        root_node_index
    };
    Ok(CallGraph {
        call_graph,
        root_node_index,
        root_scope_id,
    })
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
        let CallGraphNode::Compute {
            component_id, ..
        } = node else {
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
            "Fallible nodes should have either none, one or two children. This is not the case for node {:?}.\nGraph:\n{}",
            node_index,
            call_graph.debug_dot(component_db, computation_db)
        );

        let first_child_node_index = child_node_indexes[0];
        let second_child_node_index = child_node_indexes[1];
        for idx in [first_child_node_index, second_child_node_index] {
            match &call_graph[idx] {
                CallGraphNode::Compute { component_id, .. } => {
                    assert!([ok_match_id, err_match_id].contains(&component_id));
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
/// from a reference input type rather than a `SharedBorrow` edge from a non-reference input type.
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
        let CallGraphNode::InputParameter { source, type_: input_type } = node else { continue; };
        match input_type {
            ResolvedType::Reference(_) => continue,
            _ => {
                let is_only_borrowed = call_graph
                    .edges_directed(node_index, Direction::Outgoing)
                    .all(|edge| edge.weight() == &CallGraphEdgeMetadata::SharedBorrow);
                if is_only_borrowed {
                    let reference_input_type = ResolvedType::Reference(TypeReference {
                        is_mutable: false,
                        is_static: false,
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
/// The direction of the edge is from the node that provides the input to the node that requires it.
///
/// The type of edge determines *how* the input is consumed.
pub(crate) enum CallGraphEdgeMetadata {
    /// The input is consumed by value—e.g. `String` in `fn handler(input: String)`.
    Move,
    /// The dependent requires a shared reference to the input type —e.g. `&MyStruct` in
    /// `fn handler(input: &MyStruct)`.
    SharedBorrow,
}

impl From<ConsumptionMode> for CallGraphEdgeMetadata {
    fn from(value: ConsumptionMode) -> Self {
        match value {
            ConsumptionMode::Move => CallGraphEdgeMetadata::Move,
            ConsumptionMode::SharedBorrow => CallGraphEdgeMetadata::SharedBorrow,
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
    component_id: ComponentId,
    neighbour: Option<VisitorNeighbour>,
}

#[derive(Debug, Eq, PartialEq, Hash)]
enum VisitorNeighbour {
    Parent(NodeIndex, CallGraphEdgeMetadata),
    Child(NodeIndex, CallGraphEdgeMetadata),
}

impl VisitorStackElement {
    /// A short-cut to add a node without a parent to the visitor stack.
    fn orphan(component_id: ComponentId) -> Self {
        Self {
            component_id,
            neighbour: None,
        }
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
                            Computation::FrameworkItem(i) => {
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
                                Computation::FrameworkItem(i) => {
                                    format!("{i:?}")
                                }
                            };
                            format!("{component_label} (Component ix: {component_id:?})")
                        }
                        CallGraphNode::InputParameter { type_, .. } => {
                            format!("{type_:?}")
                        }
                        CallGraphNode::MatchBranching => "`match`".to_string(),
                    };
                    format!("label = \"{label} (Node ix: {index:?})\"")
                },
            )
        )
    }
}
