use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};

use ahash::{HashMap, HashMapExt};
use bimap::BiHashMap;
use fixedbitset::FixedBitSet;
use guppy::PackageId;
use indexmap::IndexMap;
use itertools::Itertools;
use petgraph::Direction;
use petgraph::graph::NodeIndex;
use petgraph::prelude::{DfsPostOrder, EdgeRef};
use petgraph::visit::{Dfs, IntoNeighborsDirected, Reversed, VisitMap, Visitable};
use proc_macro2::{Ident, TokenStream};
use quote::{ToTokens, quote};
use syn::ItemFn;

use crate::compiler::analyses::call_graph::core_graph::{CallGraphEdgeMetadata, RawCallGraph};
use crate::compiler::analyses::call_graph::{CallGraphNode, OrderedCallGraph};
use crate::compiler::analyses::components::ComponentDb;
use crate::compiler::analyses::components::HydratedComponent;
use crate::compiler::analyses::computations::ComputationDb;
use crate::compiler::codegen_utils;
use crate::compiler::codegen_utils::{Fragment, VariableNameGenerator};
use crate::compiler::component::Constructor;
use crate::compiler::computation::{Computation, MatchResultVariant};
use crate::language::ResolvedType;

/// Generate the dependency closure of the [`OrderedCallGraph`]'s root callable.
///
/// If the generation is successful, it returns a free function (an [`ItemFn`]) that wraps the
/// underlying root callable.
pub(crate) fn codegen_callable_closure(
    call_graph: &OrderedCallGraph,
    package_id2name: &BiHashMap<PackageId, String>,
    component_db: &ComponentDb,
    computation_db: &ComputationDb,
) -> Result<ItemFn, anyhow::Error> {
    let input_parameter_types = call_graph.required_input_types();
    let mut variable_generator = VariableNameGenerator::new();
    // Assign a unique parameter name to each input parameter type.
    let parameter_bindings: HashMap<ResolvedType, Ident> = input_parameter_types
        .iter()
        .map(|type_| {
            let parameter_name = variable_generator.generate();
            (type_.to_owned(), parameter_name)
        })
        .collect();
    let body = codegen_callable_closure_body(
        call_graph,
        &parameter_bindings,
        package_id2name,
        component_db,
        computation_db,
        &mut variable_generator,
    )?;

    let function = {
        let inputs = input_parameter_types.into_iter().map(|mut type_| {
            let variable_name = &parameter_bindings[&type_];
            // We can set all the non-'static lifetimes to implied (i.e. '_) in function signatures.
            let original2renamed = type_
                .named_lifetime_parameters()
                .into_iter()
                .map(|l| (l, "_".to_string()))
                .collect();
            type_.rename_lifetime_parameters(&original2renamed);
            let variable_type = type_.syn_type(package_id2name);
            quote! { #variable_name: #variable_type }
        });
        let component_id = match &call_graph.call_graph[call_graph.root_node_index] {
            CallGraphNode::Compute { component_id, .. } => component_id,
            n => {
                dbg!(n);
                unreachable!()
            }
        };
        let output_type = component_db
            .hydrated_component(*component_id, computation_db)
            .output_type()
            .unwrap()
            .syn_type(package_id2name);
        syn::parse2(quote! {
            pub async fn handler(#(#inputs),*) -> #output_type {
                #body
            }
        })
        .unwrap()
    };
    Ok(function)
}

#[derive(Clone, Debug)]
/// A visitor that traverses a portion of the call graph, starting from a particular node.
/// In particular:
///
/// - It yields nodes according to the total order established by the node indices.
///   Nodes with a lower index are yielded first.
/// - It only visits nodes:
///   - connected to the starting node, disregarding the directionality of the edges.
///   - that can reach at least one sink node that's also reachable from the starting node,
///     taking into account the directionality of the edges.
///
/// These rules are meant to identify a section of the call graph that can be executed
/// with no branching—i.e. a "basic block", adopting the terminology of
/// [control flow graphs](https://en.wikipedia.org/wiki/Control-flow_graph).
/// We don't actually build a CFG, but we try to approximate the concept using this
/// visitor.
struct BasicBlockVisitor {
    /// The start node must either be:
    /// - a branching node
    /// - a sink node
    start: NodeIndex,
    /// The nodes yet to be visited, ordered according to their position.
    /// The nodes with the lowest position are visited first, hence the "reversal".
    to_be_visited: BinaryHeap<Reverse<UnvisitedNode>>,
    /// The map of discovered nodes
    discovered: FixedBitSet,
    /// The map of finished nodes
    finished: FixedBitSet,
    /// The map that assigns to each node the set of sinks reachable from it.
    id2sink_ids: HashMap<NodeIndex, BTreeSet<NodeIndex>>,
}

#[derive(Debug, Eq, PartialEq, Clone, PartialOrd, Ord)]
struct UnvisitedNode {
    node: NodeIndex,
}

impl BasicBlockVisitor {
    /// Create a new [`PositionAwareVisitor`] using the graph's visitor map, and put
    /// `start` in the stack of nodes to visit.
    pub(crate) fn new(ordered_call_graph: &OrderedCallGraph, start_node: NodeIndex) -> Self {
        let reachability_map = compute_reachability_map(&ordered_call_graph.call_graph);
        let graph = Reversed(&ordered_call_graph.call_graph);
        let mut to_be_visited = BinaryHeap::new();
        to_be_visited.push(Reverse(UnvisitedNode { node: start_node }));
        Self {
            start: start_node,
            to_be_visited,
            discovered: graph.visit_map(),
            finished: graph.visit_map(),
            id2sink_ids: reachability_map,
        }
    }

    /// Keep the discovered and finished map, but clear the visit stack and restart
    /// the dfs from a particular node.
    pub fn move_to(&mut self, start: NodeIndex) {
        self.start = start;
        self.to_be_visited.clear();
        self.discovered.clear();
        self.to_be_visited
            .push(Reverse(UnvisitedNode { node: start }));
    }

    /// Return the next node in the traversal, or `None` if the traversal is done.
    pub fn next(&mut self, graph: Reversed<&RawCallGraph>) -> Option<NodeIndex> {
        while let Some(nx) = self.to_be_visited.peek() {
            let nx = nx.0.node;
            if self.discovered.visit(nx) {
                let max_index = self.start;
                let interesting_terminals = &self.id2sink_ids[&self.start];
                // First time visiting `nx`: Push neighbors, don't pop `nx`
                let neighbors = graph
                    .neighbors_directed(nx, Direction::Incoming)
                    .chain(graph.neighbors_directed(nx, Direction::Outgoing))
                    .filter(|&succ| {
                        succ <= max_index
                            && !self.discovered.is_visited(&succ)
                            && {
                                // Don't start visiting new branching nodes.
                                if let CallGraphNode::MatchBranching = graph.0[succ] {
                                    succ == self.start
                                } else {
                                    true
                                }
                            }
                            && interesting_terminals
                                .intersection(&self.id2sink_ids[&succ])
                                .next()
                                .is_some()
                    });
                for succ in neighbors {
                    let succ = UnvisitedNode { node: succ };
                    self.to_be_visited.push(Reverse(succ));
                }
            } else {
                self.to_be_visited.pop();
                if self.finished.visit(nx) {
                    // Second time: All reachable nodes must have been finished
                    return Some(nx);
                }
            }
        }
        None
    }
}

/// Generate the function body for the dependency closure of the [`CallGraph`]'s root callable.
///
/// See [`CallGraph`] docs for more details.
///
/// [`CallGraph`]: crate::compiler::analyses::call_graph::CallGraph
fn codegen_callable_closure_body(
    ocg: &OrderedCallGraph,
    parameter_bindings: &HashMap<ResolvedType, Ident>,
    package_id2name: &BiHashMap<PackageId, String>,
    component_db: &ComponentDb,
    computation_db: &ComputationDb,
    variable_name_generator: &mut VariableNameGenerator,
) -> Result<TokenStream, anyhow::Error> {
    let mut at_most_once_constructor_blocks = IndexMap::<NodeIndex, TokenStream>::new();
    let mut blocks = BTreeMap::<NodeIndex, Fragment>::new();
    let mut dfs = BasicBlockVisitor::new(ocg, ocg.root_node_index);
    let n_nodes = ocg.call_graph.node_count();
    let mut visited_at_least_once = FixedBitSet::with_capacity(n_nodes);
    let happy_terminal = dfs.id2sink_ids[&ocg.root_node_index].clone();
    let body = _codegen_callable_closure_body(
        &happy_terminal,
        &ocg.call_graph,
        parameter_bindings,
        package_id2name,
        component_db,
        computation_db,
        variable_name_generator,
        &mut at_most_once_constructor_blocks,
        &mut blocks,
        &mut dfs,
        &mut visited_at_least_once,
    )?;

    // Invariants:
    // 1. All nodes have been visited at least once
    {
        let mut expected = FixedBitSet::with_capacity(n_nodes);
        for i in 0..n_nodes {
            expected.insert(i);
        }
        let difference: Vec<_> = expected.difference(&visited_at_least_once).collect();
        if !difference.is_empty() {
            panic!(
                "The code generation process did not visit all nodes. {} have not been visited",
                difference.iter().join(",")
            );
        }
    }
    Ok(body)
}

/// Assign to each node in the graph the set of terminal nodes that are reachable from it.
fn compute_reachability_map(graph: &RawCallGraph) -> HashMap<NodeIndex, BTreeSet<NodeIndex>> {
    let mut reachability_map = HashMap::<NodeIndex, BTreeSet<NodeIndex>>::new();
    let terminal_nodes = graph.externals(Direction::Outgoing);
    for terminal_node in terminal_nodes {
        let mut dfs = Dfs::new(&Reversed(graph), terminal_node);
        while let Some(node) = dfs.next(&Reversed(graph)) {
            reachability_map
                .entry(node)
                .or_default()
                .insert(terminal_node);
        }
    }
    reachability_map
}

fn _codegen_callable_closure_body(
    target_terminals: &BTreeSet<NodeIndex>,
    call_graph: &RawCallGraph,
    parameter_bindings: &HashMap<ResolvedType, Ident>,
    package_id2name: &BiHashMap<PackageId, String>,
    component_db: &ComponentDb,
    computation_db: &ComputationDb,
    variable_name_generator: &mut VariableNameGenerator,
    at_most_once_constructor_blocks: &mut IndexMap<NodeIndex, TokenStream>,
    blocks: &mut BTreeMap<NodeIndex, Fragment>,
    dfs: &mut BasicBlockVisitor,
    visited_at_least_once: &mut FixedBitSet,
) -> Result<TokenStream, anyhow::Error> {
    let terminal_index = *target_terminals
        .first()
        .expect("There is no target terminal node for our traversal!");
    // We want to start the code-generation process from a `MatchBranching` node with
    // no `MatchBranching` predecessors.
    // This ensures that we don't have to look-ahead when generating code for its predecessors.
    let traversal_start_index =
        find_match_branching_ancestor(terminal_index, call_graph, &dfs.finished)
            // If there are no `MatchBranching` nodes in the ancestors sub-graph, we start from the
            // the terminal node.
            .unwrap_or(terminal_index);
    dfs.move_to(traversal_start_index);
    while let Some(current_index) = dfs.next(Reversed(call_graph)) {
        visited_at_least_once.insert(current_index.index());
        let current_node = &call_graph[current_index];
        match current_node {
            CallGraphNode::Compute { component_id, .. } => {
                let component = component_db.hydrated_component(*component_id, computation_db);
                let computation = component.computation();
                match &computation {
                    Computation::Callable(callable) => {
                        let block = codegen_utils::codegen_call_block(
                            get_node_type_inputs(
                                current_index,
                                call_graph,
                                component_db,
                                computation_db,
                            ),
                            get_node_happen_befores(current_index, call_graph),
                            callable.as_ref(),
                            blocks,
                            variable_name_generator,
                            package_id2name,
                        )?;
                        // This is the last node!
                        // We don't need to assign its value to a variable.
                        if current_index == traversal_start_index
                            ||
                            // Or if the output type is `None`, we don't need to assign its value to a variable.
                            computation.output_type().is_none()
                        {
                            blocks.insert(current_index, block);
                        } else {
                            // We bind the constructed value to a variable name and instruct
                            // all dependents to refer to the constructed value via that
                            // variable name.
                            let parameter_name = variable_name_generator.generate();
                            let is_borrowed_mutably = call_graph
                                .edges_directed(current_index, Direction::Outgoing)
                                .any(|e| e.weight() == &CallGraphEdgeMetadata::ExclusiveBorrow);
                            let maybe_mut = is_borrowed_mutably.then(|| quote! {mut});
                            let block = quote! {
                                let #maybe_mut #parameter_name = #block;
                            };
                            at_most_once_constructor_blocks.insert(current_index, block);
                            blocks
                                .insert(current_index, Fragment::VariableReference(parameter_name));
                        }
                    }
                    Computation::MatchResult(_) => {
                        // We already bound the match result to a variable name when handling
                        // its parent `MatchBranching` node.
                    }
                    Computation::PrebuiltType(_) => {
                        unreachable!("Prebuilt items should only appear as input parameters.")
                    }
                }
            }
            CallGraphNode::InputParameter {
                type_: input_type, ..
            } => {
                let parameter_name = parameter_bindings[input_type].clone();
                blocks.insert(current_index, Fragment::VariableReference(parameter_name));
            }
            CallGraphNode::MatchBranching => {
                let variants = call_graph
                    .neighbors_directed(current_index, Direction::Outgoing)
                    .collect::<Vec<_>>();
                assert_eq!(2, variants.len());
                assert_eq!(current_index, dfs.start);
                let mut ok_arm = None;
                let mut ok_binding_variable = None;
                let mut err_arm = None;
                for variant_index in variants {
                    let mut at_most_once_constructor_blocks = IndexMap::new();
                    let mut variant_name_generator = variable_name_generator.clone();
                    let match_binding_parameter_name = variant_name_generator.generate();
                    let mut variant_blocks = {
                        let mut b = blocks.clone();
                        b.insert(
                            variant_index,
                            Fragment::VariableReference(match_binding_parameter_name.clone()),
                        );
                        b
                    };
                    // This `.clone()` is **load-bearing**.
                    // The sub-graph for each match arm might have one or more nodes in common.
                    // If we don't create a new DFS for each match arm, the visitor will only
                    // pick up the shared nodes once (for the first match arm), leading to issues
                    // when generating code for the second match arm (i.e. most likely a panic).
                    let mut new_dfs = dfs.clone();
                    let mut new_target_terminals: BTreeSet<_> = target_terminals
                        .intersection(&dfs.id2sink_ids[&variant_index])
                        .copied()
                        .collect();
                    if new_target_terminals.is_empty() {
                        // We may be entering the error arm of a match, so we need to
                        // accommodate that terminal node.
                        new_target_terminals = dfs.id2sink_ids[&variant_index].clone();
                    }
                    let match_arm_body = _codegen_callable_closure_body(
                        &new_target_terminals,
                        call_graph,
                        parameter_bindings,
                        package_id2name,
                        component_db,
                        computation_db,
                        &mut variant_name_generator,
                        &mut at_most_once_constructor_blocks,
                        &mut variant_blocks,
                        &mut new_dfs,
                        visited_at_least_once,
                    )?;
                    let variant_type = match &call_graph[variant_index] {
                        CallGraphNode::Compute { component_id, .. } => {
                            match component_db.hydrated_component(*component_id, computation_db) {
                                HydratedComponent::Transformer(Computation::MatchResult(m), ..)
                                | HydratedComponent::Constructor(Constructor(
                                    Computation::MatchResult(m),
                                )) => m.variant,
                                _ => unreachable!(),
                            }
                        }
                        _ => unreachable!(),
                    };
                    match variant_type {
                        MatchResultVariant::Ok => {
                            ok_binding_variable = Some(match_binding_parameter_name.clone());
                            ok_arm = Some(match_arm_body);
                        }
                        MatchResultVariant::Err => {
                            err_arm = Some(quote! {
                                Err(#match_binding_parameter_name) => return {
                                    #match_arm_body
                                }
                            });
                        }
                    }
                }
                // We do this to make sure that the Ok arm is always before the Err arm in the
                // generated code.
                let ok_arm = ok_arm.unwrap();
                let ok_binding_variable = ok_binding_variable.unwrap();
                let err_arm = err_arm.unwrap();
                let result_node_index = call_graph
                    .neighbors_directed(current_index, Direction::Incoming)
                    .next()
                    .unwrap();
                let result_binding = &blocks[&result_node_index];
                let block = quote! {
                    {
                        let #ok_binding_variable = match #result_binding {
                            Ok(ok) => ok,
                            #err_arm
                        };
                        #ok_arm
                    }
                };
                blocks.insert(current_index, Fragment::Block(syn::parse2(block).unwrap()));
            }
        }
    }
    let body = {
        let at_most_once_constructors = at_most_once_constructor_blocks
            .iter()
            .sorted_by_key(|(k, _)| *k)
            .map(|(_, v)| v);
        let at_most_once_constructors = at_most_once_constructors.collect::<Vec<_>>();
        // Remove the wrapping block, if there is one
        let b = match &blocks[&traversal_start_index] {
            Fragment::Block(b) => {
                let s = &b.stmts;
                quote! { #(#s)* }
            }
            Fragment::Statement(b) => b.to_token_stream(),
            Fragment::VariableReference(n) => n.to_token_stream(),
        };
        quote! {
            #(#at_most_once_constructors)*
            #b
        }
    };
    Ok(body)
}

/// Returns a terminal descendant of the given node—i.e. a node that is reachable from
/// `start_index` and has no outgoing edges.
///
/// Returns `Some(node_index)` if there is an ancestor (either directly or indirectly connected
/// to `start_index`) that is a `CallGraphNode::MatchBranching` and doesn't belong to `ignore_set`.
/// `node` index won't have any ancestors that are themselves a `CallGraphNode::MatchBranching`.
///
/// Returns `None` if such an ancestor doesn't exist.
fn find_match_branching_ancestor(
    start_index: NodeIndex,
    call_graph: &RawCallGraph,
    ignore_set: &FixedBitSet,
) -> Option<NodeIndex> {
    let mut ancestors = DfsPostOrder::new(Reversed(call_graph), start_index);
    let mut candidates = Vec::new();
    while let Some(ancestor_index) = ancestors.next(Reversed(call_graph)) {
        if ancestor_index == start_index {
            continue;
        }
        if ignore_set.contains(ancestor_index.index()) {
            continue;
        }
        if let CallGraphNode::MatchBranching = &call_graph[ancestor_index] {
            candidates.push(ancestor_index);
        }
    }
    if candidates.is_empty() {
        None
    } else {
        let (_, lowest, _) = candidates.select_nth_unstable(0);
        Some(*lowest)
    }
}

/// Return the direct dependencies of a node in the call graph.
///
/// Dependencies are **ordered with respect to their `position`**—i.e.
/// the dependency with the lowest `position` is the first element in the returned iterator.
fn get_node_type_inputs<'a, 'b: 'a>(
    node_index: NodeIndex,
    call_graph: &'a RawCallGraph,
    component_db: &'b ComponentDb,
    computation_db: &'b ComputationDb,
) -> impl Iterator<Item = (NodeIndex, ResolvedType, CallGraphEdgeMetadata)> + 'a {
    call_graph
        .edges_directed(node_index, Direction::Incoming)
        .filter_map(move |edge| {
            if edge.weight() == &CallGraphEdgeMetadata::HappensBefore {
                // It's not an input parameter, so we don't care about it.
                return None;
            }
            let node = &call_graph[edge.source()];
            let type_ = match node {
                CallGraphNode::Compute { component_id, .. } => {
                    let component = component_db.hydrated_component(*component_id, computation_db);
                    match component.output_type().cloned() {
                        Some(type_) => type_,
                        None => {
                            return None;
                        }
                    }
                }
                CallGraphNode::InputParameter { type_, .. } => type_.to_owned(),
                CallGraphNode::MatchBranching => unreachable!(),
            };
            Some((edge.source(), type_, edge.weight().to_owned()))
        })
        .sorted_by_key(|(node_index, _, _)| *node_index)
}

fn get_node_happen_befores(
    node_index: NodeIndex,
    call_graph: &RawCallGraph,
) -> impl Iterator<Item = NodeIndex> + '_ {
    call_graph
        .edges_directed(node_index, Direction::Incoming)
        .filter_map(move |edge| {
            if edge.weight() != &CallGraphEdgeMetadata::HappensBefore {
                // It's an input parameter, so we don't care about it.
                return None;
            }
            Some(edge.source())
        })
        .sorted_by_key(|node_index| *node_index)
}
