use std::collections::VecDeque;

use ahash::{HashMap, HashMapExt};
use guppy::graph::PackageGraph;
use indexmap::IndexSet;
use miette::NamedSource;
use petgraph::graph::NodeIndex;
use petgraph::prelude::EdgeRef;
use petgraph::visit::NodeRef;
use petgraph::Direction;

use crate::compiler::analyses::call_graph::borrow_checker::clone::get_clone_component_id;
use crate::compiler::analyses::call_graph::core_graph::{InputParameterSource, RawCallGraph};
use crate::compiler::analyses::call_graph::{
    CallGraph, CallGraphEdgeMetadata, CallGraphNode, NumberOfAllowedInvocations,
};
use crate::compiler::analyses::components::ComponentDb;
use crate::compiler::analyses::computations::ComputationDb;
use crate::compiler::computation::Computation;
use crate::diagnostic;
use crate::diagnostic::{
    AnnotatedSnippet, CompilerDiagnostic, HelpWithSnippet, LocationExt, OptionalSourceSpanExt,
};
use crate::rustdoc::CrateCollection;

use super::copy::CopyChecker;
use super::diagnostic_helpers::suggest_wrapping_in_a_smart_pointer;

/// Scan the call graph for a specific kind of borrow-checking violation:
///
/// - node `A` consumes one its dependencies, `B`, by value;
/// - `B` is also borrowed by another node, `C`, where `C` is a descendant of `A` (i.e. there is
///   a path connecting them).
///
/// If this happens, we try to clone `B` (if its cloning strategy allows for it), otherwise we emit
/// an error.
///
/// This is the "best" kind of borrow checking violation, because we can return a very clear
/// diagnostic message to the user.
/// The more subtle kind of violations are handled by [`super::complex::complex_borrow_check`].
pub(super) fn ancestor_consumes_descendant_borrows(
    call_graph: CallGraph,
    copy_checker: &CopyChecker,
    component_db: &mut ComponentDb,
    computation_db: &mut ComputationDb,
    package_graph: &PackageGraph,
    krate_collection: &CrateCollection,
    diagnostics: &mut Vec<miette::Error>,
) -> CallGraph {
    let CallGraph {
        mut call_graph,
        root_node_index,
        root_scope_id,
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
                if copy_checker.is_copy(&call_graph, dependency_index, component_db, computation_db)
                {
                    // You can't have a "borrow after moved" error for a Copy type.
                    continue;
                }

                let clone_component_id =
                    call_graph[dependency_index].component_id().and_then(|id| {
                        get_clone_component_id(
                            &id,
                            package_graph,
                            krate_collection,
                            component_db,
                            computation_db,
                            root_scope_id,
                        )
                    });

                let Some(clone_component_id) = clone_component_id else {
                    emit_ancestor_descendant_borrow_error(
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
        root_scope_id,
    }
}

fn emit_ancestor_descendant_borrow_error(
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

    let CallGraphNode::Compute {
        component_id: borrower_component_id,
        ..
    } = call_graph[downstream_borrow_node_id]
    else {
        unreachable!()
    };
    let hydrated_borrower_component =
        component_db.hydrated_component(borrower_component_id, computation_db);
    let Computation::Callable(borrower_callable) = hydrated_borrower_component.computation() else {
        unreachable!()
    };

    let CallGraphNode::Compute {
        component_id: consuming_component_id,
        ..
    } = call_graph[consuming_node_id]
    else {
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

    let borrower_path = &borrower_callable.path;
    let consumer_path = &consumer_callable.path;
    let error_msg = format!(
        "I can't generate code that will pass the borrow checker *and* match the instructions \
        in your blueprint.\n\
        `{borrower_path}` wants to borrow `{type_:?}` but `{consumer_path}`, which is invoked \
        earlier on, consumes `{type_:?}` by value.\n\
        Since I'm not allowed to clone `{type_:?}`, I can't resolve this conflict."
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

    diagnostic = suggest_wrapping_in_a_smart_pointer(
        contended_component_id,
        component_db,
        computation_db,
        diagnostic,
    );
    diagnostics.push(diagnostic.build().into());
}
