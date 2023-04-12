use std::borrow::Cow;
use std::collections::VecDeque;

use ahash::{HashMap, HashMapExt};
use guppy::graph::PackageGraph;
use indexmap::IndexSet;
use miette::NamedSource;
use once_cell::sync::OnceCell;
use petgraph::graph::NodeIndex;
use petgraph::prelude::EdgeRef;
use petgraph::visit::NodeRef;
use petgraph::Direction;

use pavex_builder::constructor::CloningStrategy;

use crate::compiler::analyses::call_graph::core_graph::RawCallGraph;
use crate::compiler::analyses::call_graph::{
    CallGraph, CallGraphEdgeMetadata, CallGraphNode, NumberOfAllowedInvocations,
};
use crate::compiler::analyses::components::{
    ComponentDb, ComponentId, ConsumptionMode, HydratedComponent,
};
use crate::compiler::analyses::computations::ComputationDb;
use crate::compiler::computation::Computation;
use crate::compiler::utils::process_framework_path;
use crate::diagnostic;
use crate::diagnostic::{
    AnnotatedSnippet, CompilerDiagnostic, HelpWithSnippet, LocationExt, OptionalSourceSpanExt,
};
use crate::language::{
    Callable, InvocationStyle, PathType, ResolvedPath, ResolvedPathQualifiedSelf,
    ResolvedPathSegment, ResolvedType, TypeReference,
};
use crate::rustdoc::CrateCollection;

fn causal_borrow_checker(
    call_graph: CallGraph,
    component_db: &mut ComponentDb,
    computation_db: &mut ComputationDb,
    package_graph: &PackageGraph,
    krate_collection: &CrateCollection,
    diagnostics: &mut Vec<miette::Error>,
) -> CallGraph {
    let CallGraph {
        mut call_graph,
        root_node_index,
    } = call_graph;

    // We start from sinks (i.e. nodes that do not have outgoing edges) and work our way up,
    // traversing edges in the reverse direction (i.e. incoming edges).
    // The following invariant MUST hold at all times: once we visit a node, we must have
    // already visited all the nodes that are connected with it by an outgoing edge (i.e.
    // all the nodes that depend on it).
    let mut nodes_to_visit = VecDeque::from_iter(call_graph.externals(Direction::Outgoing));
    let mut visited_nodes = IndexSet::new();
    let mut downstream_borrows: HashMap<NodeIndex, IndexSet<NodeIndex>> = HashMap::new();

    while let Some(node_index) = nodes_to_visit.pop_front() {
        let mut borrowed_nodes: IndexSet<NodeIndex> = call_graph
            .neighbors_directed(node_index, Direction::Outgoing)
            .fold(IndexSet::new(), |mut acc, neighbor_index| {
                if let Some(downstream_borrows) = downstream_borrows.get(&neighbor_index) {
                    acc.extend(downstream_borrows);
                }
                acc
            });
        let dependency_edge_ids: Vec<_> = call_graph
            .edges_directed(node_index, Direction::Incoming)
            .map(|edge_ref| edge_ref.id())
            .collect();

        for edge_id in dependency_edge_ids {
            let edge_metadata = call_graph.edge_weight(edge_id).unwrap();
            let dependency_index = call_graph.edge_endpoints(edge_id).unwrap().0;
            if !visited_nodes.contains(&dependency_index) {
                nodes_to_visit.push_back(dependency_index);
            }

            if edge_metadata == &CallGraphEdgeMetadata::SharedBorrow {
                borrowed_nodes.insert(dependency_index);
                continue;
            }

            if borrowed_nodes.contains(&dependency_index) {
                let CallGraphNode::Compute { component_id, .. } =
                    call_graph[dependency_index].clone() else { continue; };
                let Some(clone_component_id) = get_clone_component_id(
                    &component_id,
                    package_graph,
                    krate_collection,
                    component_db,
                    computation_db,
                ) else {
                    emit_causal_borrow_checking_error(
                        dependency_index,
                        node_index,
                        computation_db,
                        component_db,
                        package_graph,
                        &call_graph,
                        diagnostics,
                    );
                    continue;
                };

                let clone_node_id = call_graph.add_node(CallGraphNode::Compute {
                    component_id: clone_component_id,
                    n_allowed_invocations: NumberOfAllowedInvocations::One,
                });

                // `Clone`'s signature is `fn clone(&self) -> Self`, therefore
                // we must introduce the new node with a "SharedBorrow" edge.
                call_graph.update_edge(
                    dependency_index,
                    clone_node_id,
                    CallGraphEdgeMetadata::SharedBorrow,
                );
                call_graph.update_edge(clone_node_id, node_index, CallGraphEdgeMetadata::Move);
                call_graph.remove_edge(edge_id);
            }
        }

        downstream_borrows.insert(node_index, borrowed_nodes);
        visited_nodes.insert(node_index);
    }

    CallGraph {
        call_graph,
        root_node_index,
    }
}

fn emit_causal_borrow_checking_error(
    contended_node_id: NodeIndex,
    consuming_node_id: NodeIndex,
    computation_db: &ComputationDb,
    component_db: &ComponentDb,
    package_graph: &PackageGraph,
    call_graph: &RawCallGraph,
    diagnostics: &mut Vec<miette::Error>,
) {
    // Find the downstream node that is borrowing the contended node.
    let mut downstream_borrow_node_id = None;
    let mut nodes_to_visit =
        VecDeque::from_iter(call_graph.neighbors_directed(consuming_node_id, Direction::Outgoing));
    while let Some(node_id) = nodes_to_visit.pop_front() {
        if call_graph
            .edges_directed(node_id, Direction::Incoming)
            .any(|edge_ref| edge_ref.source().id() == contended_node_id)
        {
            downstream_borrow_node_id = Some(node_id);
            break;
        };
        nodes_to_visit.extend(call_graph.neighbors_directed(node_id, Direction::Outgoing));
    }

    let downstream_borrow_node_id = downstream_borrow_node_id.unwrap();

    let CallGraphNode::Compute { component_id: borrower_component_id, .. } = call_graph[downstream_borrow_node_id] else {
        unreachable!()
    };
    let hydrated_borrower_component =
        component_db.hydrated_component(borrower_component_id, computation_db);
    let Computation::Callable(borrower_callable) = hydrated_borrower_component.computation() else {
        unreachable!()
    };

    let CallGraphNode::Compute { component_id: consuming_component_id, .. } = call_graph[consuming_node_id] else {
        unreachable!()
    };
    let hydrated_consumer_component =
        component_db.hydrated_component(consuming_component_id, computation_db);
    let Computation::Callable(consumer_callable) = hydrated_consumer_component.computation() else {
        unreachable!()
    };

    let (contended_component_id, type_) = match &call_graph[contended_node_id] {
        CallGraphNode::Compute { component_id, .. } => (
            Some(*component_id),
            component_db
                .hydrated_component(*component_id, computation_db)
                .output_type()
                .to_owned(),
        ),
        CallGraphNode::InputParameter(o) => (None, o.to_owned()),
        CallGraphNode::MatchBranching => {
            unreachable!()
        }
    };

    let borrower_path = &borrower_callable.path;
    let consumer_path = &consumer_callable.path;
    let error_msg = format!(
        "I can't generate code that will pass the borrow checker *and* match the instructions \
        in your blueprint.\n\
        `{borrower_path}` wants to borrow `{type_:?}` but `{consumer_path}`, which is invoked \
        earlier on, consumes `{type_:?}` by value.\n\
        Since we cannot clone `{type_:?}`, I can't resolve this conflict."
    );
    let dummy_source = NamedSource::new("", "");
    let mut diagnostic = CompilerDiagnostic::builder(dummy_source, anyhow::anyhow!(error_msg));

    if let Some(component_id) = contended_component_id {
        if let Some(user_component_id) = component_db.user_component_id(component_id) {
            let help_msg = format!(
                "Allow me to clone `{type_:?}` in order to satisfy the borrow checker.\n\
                You can do so by invoking `.cloning(CloningStrategy::CloneIfNecessary)` on the type returned by `.constructor`.",
            );
            let location = component_db
                .user_component_db()
                .get_location(user_component_id);
            let source = match location.source_file(package_graph) {
                Ok(s) => Some(s),
                Err(e) => {
                    diagnostics.push(e.into());
                    None
                }
            };
            let help = match source {
                None => HelpWithSnippet::new(
                    help_msg,
                    AnnotatedSnippet::new_with_labels(NamedSource::new("", ""), vec![]),
                ),
                Some(source) => {
                    let labeled_span = diagnostic::get_f_macro_invocation_span(&source, location)
                        .labeled("The constructor was registered here".into());
                    HelpWithSnippet::new(
                        help_msg,
                        AnnotatedSnippet::new_optional(source, labeled_span),
                    )
                }
            };
            diagnostic = diagnostic.help_with_snippet(help);
        }
    }

    let help = HelpWithSnippet::new(
        format!(
            "Considering changing the signature of `{consumer_path}`.\n\
            It takes `{type_:?}` by value. Would a shared reference, `&{type_:?}`, be enough?",
        ),
        AnnotatedSnippet::new_with_labels(NamedSource::new("", ""), vec![]),
    );
    diagnostic = diagnostic.help_with_snippet(help);

    let ref_counting_help = format!("If `{type_:?}` itself cannot implement `Clone`, consider wrapping it in an `std::sync::Rc` or `std::sync::Arc`.");
    let ref_counting_help = HelpWithSnippet::new(
        ref_counting_help,
        AnnotatedSnippet::new_with_labels(NamedSource::new("", ""), vec![]),
    );
    diagnostic = diagnostic.help_with_snippet(ref_counting_help);

    diagnostics.push(diagnostic.build().into());
}

/// Examine a call graph and check if it is possible to generate code that will
/// not violate the Rust borrow checker.
///
/// It will insert new "cloning" nodes into the call graph if it's necessary (i.e. to resolve
/// a borrow checker error) and possible (i.e. the type of the node implements `Clone`).
///
/// If the violations cannot be remediated, a diagnostic will be emitted.
pub(super) fn borrow_checker(
    call_graph: CallGraph,
    component_db: &mut ComponentDb,
    computation_db: &mut ComputationDb,
    package_graph: &PackageGraph,
    krate_collection: &CrateCollection,
    diagnostics: &mut Vec<miette::Error>,
) -> CallGraph {
    let call_graph = causal_borrow_checker(
        call_graph,
        component_db,
        computation_db,
        package_graph,
        krate_collection,
        diagnostics,
    );
    let CallGraph {
        mut call_graph,
        root_node_index,
    } = call_graph;

    let mut ownership_relationships = OwnershipRelationships::compute(&call_graph);

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    /// Determine what we should do when the node that we are processing wants to consume by value
    /// another node that is currently borrowed.
    enum StrategyOnBlock {
        /// Add the blocked node to the `parked_nodes` set.
        Park,
        /// Try to clone the input node that we are trying to consume, if allowed, in order to
        /// resolve the borrow checking stalemate.
        Clone,
        /// Emit an error diagnostic to the user explaining the situation and how to
        /// resolve it.
        Error,
    }

    let mut strategy_on_block = StrategyOnBlock::Park;
    let mut unblocked_any_node = false;
    let mut nodes_to_visit: IndexSet<NodeIndex> =
        IndexSet::from_iter(call_graph.externals(Direction::Outgoing));
    let mut parked_nodes = IndexSet::new();
    let mut finished_nodes = IndexSet::new();
    let mut n_parked_nodes = None;
    'fixed_point: loop {
        'visiting: while let Some(node_index) = nodes_to_visit.pop() {
            let (incoming_blocked_ids, incoming_unblocked_ids): (IndexSet<_>, IndexSet<_>) =
                call_graph
                    .neighbors_directed(node_index, Direction::Incoming)
                    .partition(|neighbour_index| {
                        let node_relationships = ownership_relationships.node(*neighbour_index);
                        node_relationships.is_consumed_by(node_index)
                            && node_relationships.is_borrowed()
                    });

            // The nodes that are not currently blocked can be visited next.
            nodes_to_visit.extend(
                incoming_unblocked_ids
                    .into_iter()
                    .filter(|neighbour_index| {
                        !(finished_nodes.contains(neighbour_index)
                            || parked_nodes.contains(neighbour_index))
                    }),
            );

            if incoming_blocked_ids.is_empty() {
                // If this node borrows from other nodes, remove it from the list of nodes that
                // borrow from it.
                ownership_relationships
                    .node(node_index)
                    .remove_all_borrows();
                finished_nodes.insert(node_index);
            } else {
                match strategy_on_block {
                    StrategyOnBlock::Park => {
                        parked_nodes.insert(node_index);
                    }
                    StrategyOnBlock::Clone => {
                        'incoming: for incoming_blocked_id in incoming_blocked_ids {
                            let CallGraphNode::Compute { component_id, .. } =
                                &call_graph[incoming_blocked_id] else { continue 'incoming; };
                            let Some(clone_component_id) = get_clone_component_id(
                                component_id,
                                package_graph,
                                krate_collection,
                                component_db,
                                computation_db,
                            ) else {
                                continue 'incoming;
                            };

                            let clone_node_id = call_graph.add_node(CallGraphNode::Compute {
                                component_id: clone_component_id,
                                n_allowed_invocations: NumberOfAllowedInvocations::One,
                            });

                            // `Clone`'s signature is `fn clone(&self) -> Self`, therefore
                            // we must introduce a `SharedBorrow` edge.
                            call_graph.update_edge(
                                incoming_blocked_id,
                                clone_node_id,
                                CallGraphEdgeMetadata::SharedBorrow,
                            );
                            call_graph.update_edge(
                                clone_node_id,
                                node_index,
                                CallGraphEdgeMetadata::Move,
                            );
                            if let Some(edge) =
                                call_graph.find_edge(incoming_blocked_id, node_index)
                            {
                                call_graph.remove_edge(edge);
                            }

                            unblocked_any_node = true;

                            // Some tedious bookkeeping, but it's necessary to keep the
                            // `node_id2*` maps consistent and drive the resolution forward.
                            ownership_relationships
                                .node(node_index)
                                .consumes(clone_node_id);
                            ownership_relationships
                                .node(incoming_blocked_id)
                                .remove_consumer(node_index);
                            ownership_relationships
                                .node(clone_node_id)
                                .borrows(incoming_blocked_id);

                            break 'incoming;
                        }

                        // We break the `while` loop here because we want to avoid
                        // excessive cloning. Instead of cloning all blocked nodes, we
                        // want to see first if the clone of the blocked node above is enough
                        // to unblock any other node.
                        parked_nodes.insert(node_index);
                        if unblocked_any_node {
                            break 'visiting;
                        }
                    }
                    StrategyOnBlock::Error => {
                        emit_borrow_checking_error(
                            incoming_blocked_ids,
                            computation_db,
                            component_db,
                            package_graph,
                            &call_graph,
                            diagnostics,
                        );
                    }
                }
            }
        }

        let current_n_parked_nodes = parked_nodes.len();
        if current_n_parked_nodes == 0 {
            // All good! We are done!
            break 'fixed_point;
        }
        if Some(current_n_parked_nodes) == n_parked_nodes {
            match strategy_on_block {
                StrategyOnBlock::Park => {
                    // We've reached a fixed point, it's time to try cloning to unblock some of
                    // those borrow checker errors.
                    strategy_on_block = StrategyOnBlock::Clone;
                }
                StrategyOnBlock::Clone => {
                    // We managed to unblock some nodes by cloning, but we still have some nodes
                    // that are parked. Let's see if the cloning allows us to make progress on
                    // some of those nodes.
                    if unblocked_any_node {
                        strategy_on_block = StrategyOnBlock::Park;
                    } else {
                        strategy_on_block = StrategyOnBlock::Error;
                    }
                }
                StrategyOnBlock::Error => {
                    // We've reached a fixed point and emitted all the relevant borrow checker errors.
                    // We are done.
                    break 'fixed_point;
                }
            }
        }
        n_parked_nodes = Some(current_n_parked_nodes);
        // Enqueue the parked nodes into the list of nodes to be visited in the next iteration.
        nodes_to_visit.extend(std::mem::take(&mut parked_nodes));
    }

    CallGraph {
        call_graph,
        root_node_index,
    }
}

/// Returns the `ComponentId` for a transformer component that calls `Clone::clone` on the
/// computation underpinning the given `component_id`.
///
/// If the component is not a constructor, it returns `None`.
fn get_clone_component_id(
    component_id: &ComponentId,
    package_graph: &PackageGraph,
    krate_collection: &CrateCollection,
    component_db: &mut ComponentDb,
    computation_db: &mut ComputationDb,
) -> Option<ComponentId> {
    // We only need to resolve this once.
    static CLONE_PATH_TYPE: OnceCell<PathType> = OnceCell::new();
    let clone = CLONE_PATH_TYPE.get_or_init(|| {
        let clone = process_framework_path("std::clone::Clone", package_graph, krate_collection);
        let ResolvedType::ResolvedPath(clone) = clone else { unreachable!() };
        clone
    });

    let HydratedComponent::Constructor(c) = component_db.hydrated_component(*component_id, computation_db)
        else { return None; };
    let output = c.output_type().to_owned();

    // We only add a cloning node if the component is not marked as `NeverClone`.
    let cloning_strategy = component_db.cloning_strategy(*component_id);
    if cloning_strategy == CloningStrategy::NeverClone {
        return None;
    }

    let clone_path = clone.resolved_path();
    let clone_segments = {
        let mut c = clone_path.segments.clone();
        c.push(ResolvedPathSegment {
            ident: "clone".into(),
            generic_arguments: vec![],
        });
        c
    };
    let type_clone_path = ResolvedPath {
        segments: clone_segments,
        qualified_self: Some(ResolvedPathQualifiedSelf {
            position: clone_path.segments.len(),
            type_: output.clone().into(),
        }),
        package_id: clone_path.package_id.clone(),
    };

    let clone_callable = Callable {
        is_async: false,
        output: Some(output.clone()),
        path: type_clone_path,
        inputs: vec![ResolvedType::Reference(TypeReference {
            is_mutable: false,
            is_static: false,
            inner: Box::new(output),
        })],
        invocation_style: InvocationStyle::FunctionCall,
        source_coordinates: None,
    };

    let clone_computation_id =
        computation_db.get_or_intern(Computation::Callable(Cow::Owned(clone_callable)));
    let clone_component_id = component_db.get_or_intern_transformer(
        clone_computation_id,
        *component_id,
        component_db.scope_id(*component_id),
        ConsumptionMode::SharedBorrow,
    );
    Some(clone_component_id)
}

/// Emit an error diagnostic to the user explaining why the borrow checker is going to be unhappy
/// and what they can do to fix it.
fn emit_borrow_checking_error(
    incoming_blocked_ids: IndexSet<NodeIndex>,
    computation_db: &ComputationDb,
    component_db: &ComponentDb,
    package_graph: &PackageGraph,
    call_graph: &RawCallGraph,
    diagnostics: &mut Vec<miette::Error>,
) {
    for incoming_blocked_id in incoming_blocked_ids {
        if let CallGraphNode::Compute { component_id, .. } = &call_graph[incoming_blocked_id] {
            if let HydratedComponent::Constructor(c) =
                component_db.hydrated_component(*component_id, computation_db)
            {
                let type_ = c.output_type();
                let error = anyhow::anyhow!(
                    "I can't generate code that will pass the borrow checker *and* match \
                    the instructions in your blueprint.\n\
                    There are a few different ways to unblock me: check out the help messages below!\n\
                    You only need to follow *one* of them."
                );
                let clone_help = if let Some(user_component_id) =
                    component_db.user_component_id(*component_id)
                {
                    let help_msg = format!(
                        "Allow me to clone `{type_:?}` in order to satisfy the borrow checker.\n\
                        You can do so by invoking `.cloning(CloningStrategy::CloneIfNecessary)` on the type returned by `.constructor`.",
                    );
                    let location = component_db
                        .user_component_db()
                        .get_location(user_component_id);
                    let source = match location.source_file(package_graph) {
                        Ok(s) => Some(s),
                        Err(e) => {
                            diagnostics.push(e.into());
                            None
                        }
                    };
                    let help = match source {
                        None => HelpWithSnippet::new(
                            help_msg,
                            AnnotatedSnippet::new_with_labels(NamedSource::new("", ""), vec![]),
                        ),
                        Some(source) => {
                            let labeled_span =
                                diagnostic::get_f_macro_invocation_span(&source, location)
                                    .labeled("The constructor was registered here".into());
                            HelpWithSnippet::new(
                                help_msg,
                                AnnotatedSnippet::new_optional(source, labeled_span),
                            )
                        }
                    };
                    Some(help)
                } else {
                    None
                };
                let use_ref_help = if let Computation::Callable(callable) = &c.0 {
                    let help_msg = format!(
                        "Considering changing the signature of `{}`.\n\
                        It takes `{type_:?}` by value. Would a shared reference, `&{type_:?}`, be enough?",
                        callable.path
                    );
                    let help = HelpWithSnippet::new(
                        help_msg,
                        AnnotatedSnippet::new_with_labels(NamedSource::new("", ""), vec![]),
                    );
                    Some(help)
                } else {
                    None
                };
                let ref_counting_help = format!("If `{type_:?}` itself cannot implement `Clone`, consider wrapping it in an `std::sync::Rc` or `std::sync::Arc`.");
                let ref_counting_help = HelpWithSnippet::new(
                    ref_counting_help,
                    AnnotatedSnippet::new_with_labels(NamedSource::new("", ""), vec![]),
                );
                let dummy_source = NamedSource::new("", "");
                let diagnostic = CompilerDiagnostic::builder(dummy_source, error)
                    .optional_help_with_snippet(use_ref_help)
                    .optional_help_with_snippet(clone_help)
                    .help_with_snippet(ref_counting_help)
                    .build();
                diagnostics.push(diagnostic.into());
            };
        }
    }
}

#[derive(Debug, Clone, Default)]
/// An helper struct to keep track of the ownership relationships between nodes in a call graph.
///
/// We remove nodes from the various maps as we traverse the graph and process them.
struct OwnershipRelationships {
    /// For each node, the set of nodes that it borrows from.
    node_id2borrowed_ids: HashMap<NodeIndex, IndexSet<NodeIndex>>,
    /// For each  node, the set of nodes that borrow from it.
    node_id2borrower_ids: HashMap<NodeIndex, IndexSet<NodeIndex>>,
    /// For each node, the set of nodes that it consumes by value.
    node_id2consumer_ids: HashMap<NodeIndex, IndexSet<NodeIndex>>,
    /// For each node, the set of nodes that consume it by value.
    node_id2consumed_ids: HashMap<NodeIndex, IndexSet<NodeIndex>>,
}

impl OwnershipRelationships {
    /// Bootstrap the relationship map from the underlying call graph.
    fn compute(call_graph: &RawCallGraph) -> Self {
        let mut self_ = Self::default();
        for edge_index in call_graph.edge_indices() {
            match call_graph[edge_index] {
                CallGraphEdgeMetadata::SharedBorrow => {
                    let (source, target) = call_graph.edge_endpoints(edge_index).unwrap();
                    self_.node(target).borrows(source);
                }
                CallGraphEdgeMetadata::Move => {
                    let (source, target) = call_graph.edge_endpoints(edge_index).unwrap();
                    self_.node(target).consumes(source);
                }
            }
        }
        self_
    }

    #[must_use]
    /// Zoom in on a single node, either to add new relationship or to query existing ones.
    fn node(&mut self, node_index: NodeIndex) -> NodeRelationships {
        NodeRelationships {
            relationships: self,
            node_index,
        }
    }
}

/// See [`OwnershipRelationships::node`] for more details.
struct NodeRelationships<'a> {
    relationships: &'a mut OwnershipRelationships,
    node_index: NodeIndex,
}

impl NodeRelationships<'_> {
    /// Add a "borrows" relationship between the current node and the given node index.
    /// It also populates the "borrower" relationship in the other direction.
    pub fn borrows(&mut self, borrowed_node_index: NodeIndex) {
        self.relationships
            .node_id2borrowed_ids
            .entry(self.node_index)
            .or_default()
            .insert(borrowed_node_index);
        self.relationships
            .node_id2borrower_ids
            .entry(borrowed_node_index)
            .or_default()
            .insert(self.node_index);
    }

    /// Add a "consumes" relationship between the current node and the given node index.
    /// It also populates the "consumer" relationship in the other direction.
    pub fn consumes(&mut self, consumed_node_index: NodeIndex) {
        self.relationships
            .node_id2consumed_ids
            .entry(self.node_index)
            .or_default()
            .insert(consumed_node_index);
        self.relationships
            .node_id2consumer_ids
            .entry(consumed_node_index)
            .or_default()
            .insert(self.node_index);
    }

    /// Returns `true` if the current node is borrowed by at least another node.
    pub fn is_borrowed(&self) -> bool {
        self.relationships
            .node_id2borrower_ids
            .get(&self.node_index)
            .map(|borrowers| !borrowers.is_empty())
            .unwrap_or(false)
    }

    /// Returns `true` if the current node is consumed by the provided node index.
    pub fn is_consumed_by(&self, consumer_index: NodeIndex) -> bool {
        self.relationships
            .node_id2consumer_ids
            .get(&self.node_index)
            .map(|consumers| consumers.contains(&consumer_index))
            .unwrap_or(false)
    }

    /// Remove all "borrows" relationships from the current node.
    /// It also removes the "borrower" relationships in the other direction.
    pub fn remove_all_borrows(&mut self) {
        self.relationships
            .node_id2borrowed_ids
            .get_mut(&self.node_index)
            .map(|borrowed| {
                for borrowed_node_index in borrowed.iter().copied() {
                    self.relationships
                        .node_id2borrower_ids
                        .get_mut(&borrowed_node_index)
                        .map(|borrowers| {
                            borrowers.remove(&self.node_index);
                        });
                }
                borrowed.clear();
            });
    }

    /// Remove a consumer relationship with respect to `consumer_index` from the current node, if one exists.
    /// It also removes the "consumed" relationship in the other direction.
    pub fn remove_consumer(&mut self, consumer_index: NodeIndex) {
        self.relationships
            .node_id2consumer_ids
            .get_mut(&self.node_index)
            .map(|consumers| {
                consumers.remove(&consumer_index);
            });
        self.relationships
            .node_id2consumed_ids
            .get_mut(&consumer_index)
            .map(|consumed| {
                consumed.remove(&self.node_index);
            });
    }
}
