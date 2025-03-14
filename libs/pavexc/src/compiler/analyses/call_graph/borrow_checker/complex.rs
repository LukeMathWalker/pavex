use indexmap::IndexSet;
use petgraph::Direction;
use petgraph::graph::NodeIndex;

use crate::compiler::analyses::call_graph::borrow_checker::clone::get_clone_component_id;
use crate::compiler::analyses::call_graph::borrow_checker::ownership_relationship::OwnershipRelationships;
use crate::compiler::analyses::call_graph::core_graph::{InputParameterSource, RawCallGraph};
use crate::compiler::analyses::call_graph::{
    CallGraph, CallGraphEdgeMetadata, CallGraphNode, NumberOfAllowedInvocations,
};
use crate::compiler::analyses::components::ComponentDb;
use crate::compiler::analyses::computations::ComputationDb;
use crate::compiler::computation::Computation;
use crate::diagnostic::{self, OptionalLabeledSpanExt};
use crate::diagnostic::{
    AnnotatedSource, CompilerDiagnostic, HelpWithSnippet, OptionalSourceSpanExt,
};
use crate::rustdoc::CrateCollection;

use super::copy::CopyChecker;
use super::diagnostic_helpers::suggest_wrapping_in_a_smart_pointer;

/// This check is more subtle than [`ancestor_consumes_descendant_borrows`].
/// It detects other kinds of issues that prevent us from generating code that passes the borrow checker.
///
/// For example, consider this call graph:
///
/// ```text
///  A   B
/// &| X |&
///  D   C
///   \ /
/// handler
/// ```
///
/// If `D` is constructed before `C`, then `A` cannot be borrowed by `C`'s constructor after it
/// has been moved to construct `D`.
/// If `C` is constructed before `D`, then `B` cannot be borrowed by `D`'s constructor after it
/// has been moved to construct `C`.
///
/// Pavex should detect this and return two errors.
///
/// The same error can play out across a larger sub-graph, which makes explaining to
/// the user what is wrong particularly challenging.
///
/// [`ancestor_consumes_descendant_borrows`]: super::move_while_borrowed::move_while_borrowed
pub(super) fn complex_borrow_check(
    call_graph: CallGraph,
    copy_checker: &CopyChecker,
    component_db: &mut ComponentDb,
    computation_db: &mut ComputationDb,
    krate_collection: &CrateCollection,
    diagnostics: &mut crate::diagnostic::DiagnosticSink,
) -> CallGraph {
    let CallGraph {
        mut call_graph,
        root_node_index,
        root_scope_id,
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
                        let mut is_blocked = node_relationships.is_consumed_by(node_index)
                            && node_relationships.is_borrowed();

                        if is_blocked
                            && copy_checker.is_copy(
                                &call_graph,
                                *neighbour_index,
                                component_db,
                                computation_db,
                            )
                        {
                            // You can't have a "used after moved" error for a Copy type.
                            is_blocked = false;
                        }

                        is_blocked
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
                            let Some(component_id) = call_graph[incoming_blocked_id].component_id()
                            else {
                                continue 'incoming;
                            };
                            let Some(clone_component_id) = get_clone_component_id(
                                &component_id,
                                krate_collection,
                                component_db,
                                computation_db,
                                root_scope_id,
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
        root_scope_id,
    }
}

/// Emit an error diagnostic to the user explaining why the borrow checker is going to be unhappy
/// and what they can do to fix it.
fn emit_borrow_checking_error(
    incoming_blocked_ids: IndexSet<NodeIndex>,
    computation_db: &ComputationDb,
    component_db: &ComponentDb,
    call_graph: &RawCallGraph,
    diagnostics: &mut crate::diagnostic::DiagnosticSink,
) {
    for incoming_blocked_id in incoming_blocked_ids {
        let (component_id, type_) = match &call_graph[incoming_blocked_id] {
            CallGraphNode::Compute { component_id, .. } => (
                Some(*component_id),
                component_db
                    .hydrated_component(*component_id, computation_db)
                    .output_type()
                    .cloned()
                    .unwrap(),
            ),
            CallGraphNode::InputParameter { type_, source } => {
                let id = if let InputParameterSource::Component(id) = source {
                    Some(*id)
                } else {
                    None
                };
                (id, type_.to_owned())
            }
            CallGraphNode::MatchBranching => {
                unreachable!()
            }
        };

        if let Some(component_id) = component_id {
            let error = anyhow::anyhow!(
                "I can't generate code that will pass the borrow checker *and* match \
                    the instructions in your blueprint.\n\
                    There are a few different ways to unblock me: check out the help messages below!\n\
                    You only need to follow *one* of them."
            );
            let mut diagnostic = CompilerDiagnostic::builder(error);

            if let Some(user_component_id) = component_db.user_component_id(component_id) {
                let help_msg = format!(
                    "Allow me to clone `{type_:?}` in order to satisfy the borrow checker.\n\
                        You can do so by invoking `.cloning(CloningStrategy::CloneIfNecessary)` on the type returned by `.constructor`.",
                );
                let location = component_db
                    .user_component_db()
                    .get_location(user_component_id);
                let help = match diagnostics.source(location) {
                    None => HelpWithSnippet::new(help_msg, AnnotatedSource::empty()),
                    Some(s) => {
                        let callable_type =
                            component_db.user_component_db()[user_component_id].kind();
                        let s = diagnostic::f_macro_span(s.source(), location)
                            .labeled(format!("The {callable_type} was registered here"))
                            .attach(s);
                        HelpWithSnippet::new(help_msg, s).normalize()
                    }
                };
                diagnostic = diagnostic.help_with_snippet(help);
            }

            if let Computation::Callable(callable) = component_db
                .hydrated_component(component_id, computation_db)
                .computation()
            {
                let help_msg = format!(
                    "Considering changing the signature of `{}`.\n\
                        It takes `{type_:?}` by value. Would a shared reference, `&{type_:?}`, be enough?",
                    callable.path
                );
                let help = HelpWithSnippet::new(help_msg, AnnotatedSource::empty());
                diagnostic = diagnostic.help_with_snippet(help);
            }

            diagnostic = suggest_wrapping_in_a_smart_pointer(
                Some(component_id),
                component_db,
                computation_db,
                diagnostic,
            );

            diagnostics.push(diagnostic.build());
        };
    }
}
