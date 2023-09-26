use std::collections::VecDeque;
use std::fmt::Write;
use std::ops::Deref;

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
use crate::compiler::analyses::components::{ComponentDb, ComponentId};
use crate::compiler::analyses::computations::ComputationDb;
use crate::compiler::computation::Computation;
use crate::diagnostic;
use crate::diagnostic::{
    AnnotatedSnippet, CompilerDiagnostic, HelpWithSnippet, LocationExt, OptionalSourceSpanExt,
};
use crate::language::ResolvedType;
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

    // We first do a forward pass to assign to each node the set of nodes that it captures
    // a reference from.
    // E.g. in `fn f(s: &str) -> Cow<'_, str>` the output type holds onto a reference to the
    // input type, so the input type is "borrowed" as long as the output type is in scope.
    let mut nodes_to_visit = VecDeque::from_iter(call_graph.externals(Direction::Incoming));
    let mut visited_nodes = IndexSet::new();
    let mut node2captured_nodes: HashMap<NodeIndex, IndexSet<NodeIndex>> = HashMap::new();

    while let Some(node_index) = nodes_to_visit.pop_front() {
        visited_nodes.insert(node_index);
        let node = &call_graph[node_index];

        let mut captured_input_types = IndexSet::new();
        if let Some(hydrated_component) = node.as_hydrated_component(component_db, computation_db) {
            if let Computation::Callable(callable) = hydrated_component.computation() {
                captured_input_types = callable
                    .inputs_that_output_borrows_from()
                    .iter()
                    .map(|&i| callable.inputs[i].clone())
                    .map(|t| {
                        if let ResolvedType::Reference(r) = t {
                            r.inner.deref().to_owned()
                        } else {
                            t
                        }
                    })
                    .collect();
            }
        }

        let dependency_edge_ids: Vec<_> = call_graph
            .edges_directed(node_index, Direction::Incoming)
            .map(|edge_ref| edge_ref.id())
            .collect();

        for edge_id in dependency_edge_ids {
            let dependency_index = call_graph.edge_endpoints(edge_id).unwrap().0;
            let dependency_node = &call_graph[dependency_index];
            let dependency_type = match dependency_node {
                CallGraphNode::Compute { component_id, .. } => {
                    let hydrated_component =
                        component_db.hydrated_component(*component_id, computation_db);
                    Some(hydrated_component.output_type().to_owned())
                }
                CallGraphNode::MatchBranching => None,
                CallGraphNode::InputParameter { type_, .. } => Some(type_.to_owned()),
            };
            if let Some(dependency_type) = dependency_type {
                if captured_input_types.contains(&dependency_type) {
                    let transitively_captured = node2captured_nodes
                        .get(&dependency_index)
                        .cloned()
                        .unwrap_or_default();
                    let held = node2captured_nodes.entry(node_index).or_default();
                    held.extend(transitively_captured);
                    held.insert(dependency_index);
                }
            }
        }

        for edge_id in call_graph
            .edges_directed(node_index, Direction::Outgoing)
            .map(|edge_ref| edge_ref.id())
        {
            let dependent_index = call_graph.edge_endpoints(edge_id).unwrap().1;
            if !visited_nodes.contains(&dependent_index) {
                nodes_to_visit.push_back(dependent_index);
            }
        }
    }

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

            if let Some(held) = node2captured_nodes.get(&dependency_index) {
                borrowed_nodes.extend(held);
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
                        &node2captured_nodes,
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
    node2captured_nodes: &HashMap<NodeIndex, IndexSet<NodeIndex>>,
    computation_db: &ComputationDb,
    component_db: &ComponentDb,
    package_graph: &PackageGraph,
    call_graph: &RawCallGraph,
    diagnostics: &mut Vec<miette::Error>,
) {
    // Find the downstream node that is borrowing the contended node.
    let mut downstream_borrow_node_id = None;
    let mut capturer_node_id = None;
    let mut nodes_to_visit =
        VecDeque::from_iter(call_graph.neighbors_directed(consuming_node_id, Direction::Outgoing));
    while let Some(node_id) = nodes_to_visit.pop_front() {
        let mut incoming_edges = call_graph.edges_directed(node_id, Direction::Incoming);
        if incoming_edges
            .clone()
            .any(|edge_ref| edge_ref.source().id() == contended_node_id)
        {
            downstream_borrow_node_id = Some(node_id);
            break;
        };
        if let Some(capturer_edge) = incoming_edges.find(|edge_ref| {
            let source_node_id = edge_ref.source().id();
            node2captured_nodes
                .get(&source_node_id)
                .map(|captured_nodes| captured_nodes.contains(&contended_node_id))
                .unwrap_or_default()
        }) {
            downstream_borrow_node_id = Some(node_id);
            capturer_node_id = Some(capturer_edge.source().id());
            break;
        };
        nodes_to_visit.extend(call_graph.neighbors_directed(node_id, Direction::Outgoing));
    }

    let Computation::Callable(borrower_callable) = call_graph[downstream_borrow_node_id.unwrap()]
        .as_hydrated_component(component_db, computation_db)
        .unwrap()
        .computation()
    else {
        unreachable!()
    };

    let Computation::Callable(consumer_callable) = call_graph[consuming_node_id]
        .as_hydrated_component(component_db, computation_db)
        .unwrap()
        .computation()
    else {
        unreachable!()
    };

    let (contended_component_id, contended_type) =
        get_component_id_and_type(call_graph, contended_node_id, computation_db, component_db);

    let borrower_path = &borrower_callable.path;
    let consumer_path = &consumer_callable.path;
    let mut error_msg =
        "I can't generate code that will pass the borrow checker *and* match the instructions \
        in your blueprint:\n"
            .to_string();

    if let Some(capturer_node_id) = capturer_node_id {
        let (_, capturer_type) =
            get_component_id_and_type(call_graph, capturer_node_id, computation_db, component_db);

        writeln!(&mut error_msg,
                 "- `{borrower_path}` wants to consume `{capturer_type:?}`\n\
             - `{capturer_type:?}` captures a reference to `{contended_type:?}`\n\
             - But `{consumer_path}`, which is invoked earlier on, consumes `{contended_type:?}` by value"
        )
            .unwrap();
    } else {
        writeln!(
            &mut error_msg,
            "- `{borrower_path}` wants to borrow `{contended_type:?}`\n\
            - but `{consumer_path}`, which is invoked earlier on, consumes `{contended_type:?}` by value"
        )
            .unwrap();
    }
    write!(
        &mut error_msg,
        "\nSince I'm not allowed to clone `{contended_type:?}`, I can't resolve this conflict."
    )
    .unwrap();

    let dummy_source = NamedSource::new("", "");
    let mut diagnostic = CompilerDiagnostic::builder(dummy_source, anyhow::anyhow!(error_msg));

    if let Some(component_id) = contended_component_id {
        if let Some(user_component_id) = component_db.user_component_id(component_id) {
            let help_msg = format!(
                "Allow me to clone `{contended_type:?}` in order to satisfy the borrow checker.\n\
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
            "Consider changing the signature of `{consumer_path}`.\n\
            It takes `{contended_type:?}` by value. Would a shared reference, `&{contended_type:?}`, be enough?",
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

fn get_component_id_and_type(
    call_graph: &RawCallGraph,
    node_index: NodeIndex,
    computation_db: &ComputationDb,
    component_db: &ComponentDb,
) -> (Option<ComponentId>, ResolvedType) {
    match &call_graph[node_index] {
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
    }
}
