use std::collections::VecDeque;
use std::fmt::Write;
use std::ops::Deref;

use ahash::{HashMap, HashMapExt};
use indexmap::IndexSet;
use petgraph::Direction;
use petgraph::graph::{EdgeIndex, NodeIndex};
use petgraph::prelude::{DfsPostOrder, EdgeRef};
use petgraph::visit::NodeRef;

use crate::compiler::analyses::call_graph::borrow_checker::clone::get_clone_component_id;
use crate::compiler::analyses::call_graph::core_graph::{InputParameterSource, RawCallGraph};
use crate::compiler::analyses::call_graph::{
    CallGraph, CallGraphEdgeMetadata, CallGraphNode, NumberOfAllowedInvocations,
};
use crate::compiler::analyses::components::{ComponentDb, ComponentId};
use crate::compiler::analyses::computations::ComputationDb;
use crate::compiler::analyses::user_components::ScopeId;
use crate::compiler::computation::Computation;
use crate::diagnostic::{self, OptionalLabeledSpanExt};
use crate::diagnostic::{
    AnnotatedSource, CompilerDiagnostic, HelpWithSnippet, OptionalSourceSpanExt,
};
use crate::language::ResolvedType;
use crate::rustdoc::CrateCollection;

use super::copy::CopyChecker;
use super::diagnostic_helpers::suggest_wrapping_in_a_smart_pointer;

/// Scan the call graph for a specific kind of borrow-checking violation:
///
/// - node `A` consumes one its dependencies, `B`, by value;
/// - `B` is either borrowed (e.g. via a capture `D<'_>`) or will have to be borrowed by a descendant of `A`.
///
/// If this happens, we try to clone `B` (if its cloning strategy allows for it), otherwise we emit
/// an error.
///
/// This is the "best" kind of borrow checking violation, because we can return a very clear
/// diagnostic message to the user.
/// The more subtle kinds of violations are handled by [`super::complex::complex_borrow_check`].
///
/// This also checks for the case where a mutable borrow is attempted while an immutable borrow is
/// still active.
pub(super) fn move_while_borrowed(
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

        let mut directly_borrowed = IndexSet::new();
        let mut captured = IndexSet::new();
        if let Some(hydrated_component) = node.as_hydrated_component(component_db, computation_db) {
            if let Computation::Callable(callable) = hydrated_component.computation() {
                directly_borrowed = callable
                    .inputs_that_output_borrows_immutably_from()
                    .iter()
                    .map(|&i| {
                        let t = &callable.inputs[i];
                        if let ResolvedType::Reference(r) = t {
                            r.inner.deref().to_owned()
                        } else {
                            t.to_owned()
                        }
                    })
                    .collect();
                captured = callable
                    .inputs_with_lifetime_tied_with_output()
                    .iter()
                    .map(|&i| {
                        let t = &callable.inputs[i];
                        if let ResolvedType::Reference(r) = t {
                            r.inner.deref().to_owned()
                        } else {
                            t.to_owned()
                        }
                    })
                    .collect();
                #[cfg(debug_assertions)]
                {
                    assert!(directly_borrowed.is_subset(&captured));
                }
            }
        }

        let dependency_edge_ids: Vec<_> = call_graph
            .edges_directed(node_index, Direction::Incoming)
            .filter_map(|edge_ref| {
                // We only care about type dependencies, not timing relationships.
                if let CallGraphEdgeMetadata::HappensBefore = edge_ref.weight() {
                    None
                } else {
                    Some(edge_ref.id())
                }
            })
            .collect();

        'inner: for edge_id in dependency_edge_ids {
            let dependency_index = call_graph.edge_endpoints(edge_id).unwrap().0;
            let dependency_node = &call_graph[dependency_index];
            let dependency_type = match dependency_node {
                CallGraphNode::Compute { component_id, .. } => {
                    let hydrated_component =
                        component_db.hydrated_component(*component_id, computation_db);
                    hydrated_component.output_type().cloned()
                }
                CallGraphNode::MatchBranching => None,
                CallGraphNode::InputParameter { type_, .. } => Some(type_.to_owned()),
            };
            let Some(dependency_type) = dependency_type else {
                continue 'inner;
            };
            if captured.contains(&dependency_type) {
                // The capture relationship is transitive:
                // if `A` captures `B` and `B` captures `C`, then `A` also captures `C`.
                // We update `node2captured_nodes` to reflect this.
                let transitively_captured = node2captured_nodes
                    .get(&dependency_index)
                    .cloned()
                    .unwrap_or_default();
                node2captured_nodes
                    .entry(node_index)
                    .or_default()
                    .extend(transitively_captured);
            }
            if directly_borrowed.contains(&dependency_type) {
                node2captured_nodes
                    .entry(node_index)
                    .or_default()
                    .insert(dependency_index);
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
    // Any source node works as a starting point for our DfS.
    let source_id = call_graph.externals(Direction::Incoming).next().unwrap();
    let mut dfs = DfsPostOrder::new(&call_graph, source_id);
    let mut node2borrows: HashMap<NodeIndex, IndexSet<NodeIndex>> = HashMap::new();

    while let Some(node_index) = dfs.next(&call_graph) {
        let borrowed_later: IndexSet<NodeIndex> = call_graph
            .neighbors_directed(node_index, Direction::Outgoing)
            .fold(IndexSet::new(), |mut acc, neighbor_index| {
                if let Some(downstream_borrows) = node2borrows.get(&neighbor_index) {
                    acc.extend(downstream_borrows);
                }
                acc
            });
        let mut borrowed_immutably_now: IndexSet<NodeIndex> = IndexSet::new();
        let mut borrowed_mutably_now: IndexSet<NodeIndex> = IndexSet::new();

        let dependency_edge_ids: Vec<_> = call_graph
            .edges_directed(node_index, Direction::Incoming)
            .map(|edge_ref| edge_ref.id())
            .collect();

        dependency_edge_ids.iter().for_each(|edge_id| {
            let edge_metadata = call_graph.edge_weight(*edge_id).unwrap();
            let dependency_index = call_graph.edge_endpoints(*edge_id).unwrap().0;

            if let Some(captured) = node2captured_nodes.get(&dependency_index) {
                borrowed_immutably_now.extend(captured);
            }
            match edge_metadata {
                CallGraphEdgeMetadata::HappensBefore | CallGraphEdgeMetadata::Move => {}
                CallGraphEdgeMetadata::ExclusiveBorrow => {
                    borrowed_mutably_now.insert(dependency_index);
                }
                CallGraphEdgeMetadata::SharedBorrow => {
                    borrowed_immutably_now.insert(dependency_index);
                }
            }
        });

        'dependencies: for edge_id in dependency_edge_ids {
            let dependency_index = call_graph.edge_endpoints(edge_id).unwrap().0;
            match call_graph.edge_weight(edge_id).unwrap() {
                CallGraphEdgeMetadata::Move => {
                    if borrowed_immutably_now.contains(&dependency_index)
                        || borrowed_later.contains(&dependency_index)
                    {
                        try_clone(
                            &mut call_graph,
                            node_index,
                            edge_id,
                            copy_checker,
                            &node2captured_nodes,
                            component_db,
                            computation_db,
                            krate_collection,
                            root_scope_id,
                            diagnostics,
                        )
                    }
                }
                CallGraphEdgeMetadata::ExclusiveBorrow => {
                    if borrowed_immutably_now.contains(&dependency_index)
                        || borrowed_later.contains(&dependency_index)
                    {
                        emit_tried_to_borrow_mut_while_borrowed_immutably(
                            dependency_index,
                            node_index,
                            &node2captured_nodes,
                            computation_db,
                            component_db,
                            &call_graph,
                            diagnostics,
                        )
                    }
                }
                CallGraphEdgeMetadata::SharedBorrow | CallGraphEdgeMetadata::HappensBefore => {
                    continue 'dependencies;
                }
            }
        }

        let mut borrowed = borrowed_immutably_now;
        borrowed.extend(&borrowed_mutably_now);
        borrowed.extend(&borrowed_later);
        node2borrows.insert(node_index, borrowed);
        visited_nodes.insert(node_index);

        // Why is this necessary?
        // We use `StableGraph` for our call graphs. For `StableGraph`, `DfsPostOrder` relies
        // on `FixedBitSet` as the storage data structure for both discovered and finished
        // nodes. The bit set is initialized with a fixed capacity equal to the number of nodes in
        // the graph _at the time of its creation_. If we add new nodes to the graph after the
        // bit set has been initialized, `petgraph` doesn't automatically grow the bit set capacity,
        // causing a panic when we inevitably go past the initial capacity.
        // To avoid this, we manually resize the bit set here, at the end of each
        // iteration of the loop, before calling `dfs.next()`.
        dfs.discovered.grow(call_graph.node_count());
        dfs.finished.grow(call_graph.node_count());
    }

    CallGraph {
        call_graph,
        root_node_index,
        root_scope_id,
    }
}

/// Try adding a `Clone` node if it's possible and necessary.
/// Emit an error otherwise.
fn try_clone(
    call_graph: &mut RawCallGraph,
    node_index: NodeIndex,
    edge_id: EdgeIndex,
    copy_checker: &CopyChecker,
    node2captured_nodes: &HashMap<NodeIndex, IndexSet<NodeIndex>>,
    component_db: &mut ComponentDb,
    computation_db: &mut ComputationDb,
    krate_collection: &CrateCollection,
    root_scope_id: ScopeId,
    diagnostics: &mut crate::diagnostic::DiagnosticSink,
) {
    let dependency_index = call_graph.edge_endpoints(edge_id).unwrap().0;
    if copy_checker.is_copy(call_graph, dependency_index, component_db, computation_db) {
        // You can't have a "borrow after moved" error for a Copy type.
        return;
    }

    let clone_component_id = call_graph[dependency_index].component_id().and_then(|id| {
        get_clone_component_id(
            &id,
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
            node2captured_nodes,
            computation_db,
            component_db,
            call_graph,
            diagnostics,
        );
        return;
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

fn emit_ancestor_descendant_borrow_error(
    contended_node_id: NodeIndex,
    consuming_node_id: NodeIndex,
    node2captured_nodes: &HashMap<NodeIndex, IndexSet<NodeIndex>>,
    computation_db: &ComputationDb,
    component_db: &ComponentDb,
    call_graph: &RawCallGraph,
    diagnostics: &mut crate::diagnostic::DiagnosticSink,
) {
    // Find the node that is borrowing the contended node.
    let mut downstream_borrow_node_id = None;
    let mut capturer_node_id = None;
    let mut nodes_to_visit = VecDeque::from_iter([consuming_node_id]);
    while let Some(node_id) = nodes_to_visit.pop_front() {
        let mut incoming_edges = call_graph.edges_directed(node_id, Direction::Incoming);
        if incoming_edges.clone().any(|edge_ref| {
            edge_ref.weight() == &CallGraphEdgeMetadata::SharedBorrow
                && edge_ref.source().id() == contended_node_id
        }) {
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

    let qualifier = if consumer_path == borrower_path {
        format!("But, at the same time, `{consumer_path}` consumes ")
    } else {
        format!("But, earlier on, `{consumer_path}` consumed ")
    };
    if let Some(capturer_node_id) = capturer_node_id {
        let (_, capturer_type) =
            get_component_id_and_type(call_graph, capturer_node_id, computation_db, component_db);

        writeln!(
            &mut error_msg,
            "- `{borrower_path}` wants to consume `{capturer_type:?}`\n\
             - `{capturer_type:?}` captures a reference to `{contended_type:?}`\n\
             - {qualifier}`{contended_type:?}` by value"
        )
        .unwrap();
    } else {
        writeln!(
            &mut error_msg,
            "- `{borrower_path}` wants to borrow `{contended_type:?}`\n\
            - {qualifier}`{contended_type:?}` by value"
        )
        .unwrap();
    }
    write!(
        &mut error_msg,
        "\nSince I'm not allowed to clone `{contended_type:?}`, I can't resolve this conflict."
    )
    .unwrap();

    let mut diagnostic = CompilerDiagnostic::builder(anyhow::anyhow!(error_msg));

    if let Some(component_id) = contended_component_id {
        if let Some(user_component_id) = component_db.user_component_id(component_id) {
            let help_msg = format!(
                "Allow me to clone `{contended_type:?}` in order to satisfy the borrow checker.\n\
                You can do so by invoking `.clone_if_necessary()` after having registered your constructor.",
            );
            let location = component_db
                .user_component_db()
                .get_location(user_component_id);
            let help = match diagnostics.source(location) {
                None => HelpWithSnippet::new(help_msg, AnnotatedSource::empty()),
                Some(s) => {
                    let s = diagnostic::f_macro_span(s.source(), location)
                        .labeled("The constructor was registered here".into())
                        .attach(s);
                    HelpWithSnippet::new(help_msg, s.normalize())
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
        AnnotatedSource::empty(),
    );
    diagnostic = diagnostic.help_with_snippet(help);

    diagnostic = suggest_wrapping_in_a_smart_pointer(
        contended_component_id,
        component_db,
        computation_db,
        diagnostic,
    );
    diagnostics.push(diagnostic.build());
}

fn emit_tried_to_borrow_mut_while_borrowed_immutably(
    contended_node_id: NodeIndex,
    mut_borrower_id: NodeIndex,
    node2captured_nodes: &HashMap<NodeIndex, IndexSet<NodeIndex>>,
    computation_db: &ComputationDb,
    component_db: &ComponentDb,
    call_graph: &RawCallGraph,
    diagnostics: &mut crate::diagnostic::DiagnosticSink,
) {
    // Find the node that is borrowing the contended node.
    let mut borrower_node_id = None;
    let mut capturer_node_id = None;
    let mut nodes_to_visit = VecDeque::from_iter([mut_borrower_id]);
    while let Some(node_id) = nodes_to_visit.pop_front() {
        let mut incoming_edges = call_graph.edges_directed(node_id, Direction::Incoming);
        if incoming_edges.clone().any(|edge_ref| {
            edge_ref.weight() == &CallGraphEdgeMetadata::SharedBorrow
                && edge_ref.source().id() == contended_node_id
        }) {
            borrower_node_id = Some(node_id);
            break;
        };
        if let Some(capturer_edge) = incoming_edges.find(|edge_ref| {
            let source_node_id = edge_ref.source().id();
            node2captured_nodes
                .get(&source_node_id)
                .map(|captured_nodes| captured_nodes.contains(&contended_node_id))
                .unwrap_or_default()
        }) {
            borrower_node_id = Some(node_id);
            capturer_node_id = Some(capturer_edge.source().id());
            break;
        };
        nodes_to_visit.extend(call_graph.neighbors_directed(node_id, Direction::Outgoing));
    }

    let Computation::Callable(borrower_callable) = call_graph[borrower_node_id.unwrap()]
        .as_hydrated_component(component_db, computation_db)
        .unwrap()
        .computation()
    else {
        unreachable!()
    };

    let Computation::Callable(mut_borrower_callable) = call_graph[mut_borrower_id]
        .as_hydrated_component(component_db, computation_db)
        .unwrap()
        .computation()
    else {
        unreachable!()
    };

    let (_, contended_type) =
        get_component_id_and_type(call_graph, contended_node_id, computation_db, component_db);

    let borrower_path = &borrower_callable.path;
    let mut_borrower_path = &mut_borrower_callable.path;
    let mut error_msg =
        "I can't generate code that will pass the borrow checker *and* match the instructions \
        in your blueprint:\n"
            .to_string();

    let qualifier = if mut_borrower_path == borrower_path {
        "But, at the same time, "
    } else {
        "But, earlier on, "
    };
    if let Some(capturer_node_id) = capturer_node_id {
        let (_, capturer_type) =
            get_component_id_and_type(call_graph, capturer_node_id, computation_db, component_db);

        writeln!(
            &mut error_msg,
            "- `{borrower_path}` wants to consume `{capturer_type:?}`\n\
             - `{capturer_type:?}` captures a reference to `{contended_type:?}`\n\
             - {qualifier}`{mut_borrower_path}` takes `&mut {contended_type:?}` as input"
        )
        .unwrap();
    } else {
        writeln!(
            &mut error_msg,
            "- `{borrower_path}` wants to borrow `{contended_type:?}`\n\
            - {qualifier}`{mut_borrower_path}` takes `&mut {contended_type:?}` as input"
        )
        .unwrap();
    }
    write!(
        &mut error_msg,
        "\nYou can't borrow a type mutably while an immutable reference to the same type is still active. I can't resolve this conflict."
    )
        .unwrap();

    let help = HelpWithSnippet::new(
        format!(
            "Consider changing the signature of `{mut_borrower_path}`.\n\
            It takes a mutable reference to `{contended_type:?}`. Would a shared reference, `&{contended_type:?}`, be enough?",
        ),
        AnnotatedSource::empty(),
    );
    diagnostics.push(
        CompilerDiagnostic::builder(anyhow::anyhow!(error_msg))
            .help_with_snippet(help)
            .build(),
    );
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
    }
}
