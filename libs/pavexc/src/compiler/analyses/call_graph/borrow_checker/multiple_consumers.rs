use std::collections::BTreeSet;

use ahash::{HashSet, HashSetExt};
use indexmap::IndexSet;
use petgraph::Outgoing;
use petgraph::algo::has_path_connecting;
use petgraph::prelude::EdgeRef;
use petgraph::stable_graph::NodeIndex;

use crate::compiler::analyses::call_graph::borrow_checker::clone::get_clone_component_id;
use crate::compiler::analyses::call_graph::core_graph::{InputParameterSource, RawCallGraph};
use crate::compiler::analyses::call_graph::{
    CallGraph, CallGraphEdgeMetadata, CallGraphNode, NumberOfAllowedInvocations,
};
use crate::compiler::analyses::components::ComponentDb;
use crate::compiler::analyses::computations::ComputationDb;
use crate::compiler::computation::Computation;
use crate::diagnostic::{
    self, AnnotatedSource, CompilerDiagnostic, HelpWithSnippet, OptionalLabeledSpanExt,
    OptionalSourceSpanExt,
};
use crate::language::ResolvedType;
use crate::rustdoc::CrateCollection;

use super::copy::CopyChecker;
use super::diagnostic_helpers::suggest_wrapping_in_a_smart_pointer;

/// Scan the call graph for a specific kind of borrow-checking violation: node `A` is consumed
/// by value by two or more nodes.
///
/// If this happens, we try to clone `A` (if its cloning strategy allows for it), otherwise we emit
/// an error.
pub(super) fn multiple_consumers(
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

    let sink_ids = call_graph.externals(Outgoing).collect::<Vec<_>>();
    let indices = call_graph.node_indices().collect::<Vec<_>>();
    for node_id in indices {
        let consumer_ids: Vec<_> = call_graph
            .edges_directed(node_id, Outgoing)
            .filter_map(|edge| {
                if *edge.weight() == CallGraphEdgeMetadata::Move {
                    Some(edge.target().to_owned())
                } else {
                    None
                }
            })
            .collect();
        if consumer_ids.len() > 1 {
            // You can't have a "used after moved" error for a Copy type.
            if copy_checker.is_copy(&call_graph, node_id, component_db, computation_db)
            // Even though `&mut` references are not `Copy`, "used after move" is not
            // the right error message for them.
            // They belong to the `move_while_borrowed` category. So we skip them here.
            || is_ref(node_id, &call_graph, component_db, computation_db)
            {
                continue;
            }

            // We have multiple consumers that want to take ownership of the
            // value.
            // This *could* be fine, if those consumers are never invoked in the same control flow
            // branch. Each sink in the graph maps to a unique path through the control flow graph:
            // two nodes are on the same control branch if they can reach the same sink node.
            let mut competing_consumer_sets = IndexSet::new();
            for sink_id in &sink_ids {
                let node_ids = consumer_ids
                    .iter()
                    .filter(|node_id| has_path_connecting(&call_graph, **node_id, *sink_id, None))
                    .copied()
                    .collect::<BTreeSet<_>>();
                // We only care about control flow paths that have more than one consumer, since
                // *those* are the ones that are violating the borrow checker.
                if node_ids.len() > 1 {
                    competing_consumer_sets.insert(node_ids);
                }
            }
            if competing_consumer_sets.is_empty() {
                // All consumers are on different control flow branches, so all good.
                continue;
            }

            let clone_component_id = call_graph[node_id].component_id().and_then(|id| {
                get_clone_component_id(
                    &id,
                    krate_collection,
                    component_db,
                    computation_db,
                    root_scope_id,
                )
            });
            let Some(clone_component_id) = clone_component_id else {
                // We want each error to mention the nodes that are *actually* competing for the
                // same value in a way that violates the borrow checker.
                // A potential improvement here would be to capture, in the error message, the
                // control flow path where the competing consumers are invoked.
                for competing_consumer_set in competing_consumer_sets {
                    emit_multiple_consumers_error(
                        node_id,
                        competing_consumer_set,
                        computation_db,
                        component_db,
                        &call_graph,
                        diagnostics,
                    );
                }
                continue;
            };

            let mut node_id2was_cloned = HashSet::new();
            for competing_consumer_set in competing_consumer_sets {
                // For each competing set of N consumers, we only need to insert N-1 clones, because
                // the last consumer can simply move the value.
                // Since a consumer can be in multiple sets, we need to keep track of which consumers
                // have already been cloned to avoid redundant clones.
                let ids = competing_consumer_set
                    .into_iter()
                    .filter(|id| !node_id2was_cloned.contains(id))
                    .collect::<Vec<_>>();
                if ids.len() <= 1 {
                    continue;
                }
                let (_last, other_ids) = ids.split_last().unwrap();

                for consumer_id in other_ids {
                    let edge_id = call_graph.find_edge(node_id, *consumer_id).unwrap();

                    let clone_node_id = call_graph.add_node(CallGraphNode::Compute {
                        component_id: clone_component_id,
                        n_allowed_invocations: NumberOfAllowedInvocations::One,
                    });

                    // `Clone`'s signature is `fn clone(&self) -> Self`, therefore
                    // we must introduce the new node with a "SharedBorrow" edge.
                    call_graph.update_edge(
                        node_id,
                        clone_node_id,
                        CallGraphEdgeMetadata::SharedBorrow,
                    );
                    call_graph.update_edge(
                        clone_node_id,
                        *consumer_id,
                        CallGraphEdgeMetadata::Move,
                    );
                    call_graph.remove_edge(edge_id);

                    node_id2was_cloned.insert(*consumer_id);
                }
            }
        }
    }
    CallGraph {
        call_graph,
        root_node_index,
        root_scope_id,
    }
}

fn is_ref(
    node_id: NodeIndex,
    call_graph: &RawCallGraph,
    component_db: &ComponentDb,
    computation_db: &ComputationDb,
) -> bool {
    let _is_ref = |ty_| matches!(ty_, &ResolvedType::Reference(..));
    match &call_graph[node_id] {
        CallGraphNode::Compute { component_id, .. } => {
            let component = component_db.hydrated_component(*component_id, computation_db);
            let Some(output_type) = component.output_type() else {
                // `()` is not a reference.
                return false;
            };
            _is_ref(output_type)
        }
        CallGraphNode::MatchBranching => false,
        CallGraphNode::InputParameter { type_, .. } => _is_ref(type_),
    }
}

fn emit_multiple_consumers_error(
    consumed_node_id: NodeIndex,
    consuming_node_ids: BTreeSet<NodeIndex>,
    computation_db: &ComputationDb,
    component_db: &ComponentDb,
    call_graph: &RawCallGraph,
    diagnostics: &mut crate::diagnostic::DiagnosticSink,
) {
    use std::fmt::Write as _;

    let (consumed_component_id, type_) = match &call_graph[consumed_node_id] {
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

    let n_consumers = consuming_node_ids.len();
    let mut consuming_snippets = vec![];
    let mut error_msg = format!(
        "I can't generate code that will pass the borrow checker *and* match the instructions \
        in your blueprint.\n\
        There are {n_consumers} components that take `{type_:?}` as an input parameter, consuming it by value:\n"
    );

    for consuming_node_id in &consuming_node_ids {
        let Some(component_id) = &call_graph[*consuming_node_id].component_id() else {
            continue;
        };
        let component_id = component_db
            .derived_from(component_id)
            .unwrap_or(*component_id);
        let Computation::Callable(callable) = component_db
            .hydrated_component(component_id, computation_db)
            .computation()
        else {
            continue;
        };
        let user_component_id = component_db.user_component_id(component_id);

        write!(&mut error_msg, "- `{}`", callable.path).unwrap();
        match user_component_id {
            None => writeln!(&mut error_msg),
            Some(user_component_id) => {
                let callable_type = component_db.user_component_db()[user_component_id].kind();
                writeln!(&mut error_msg, ", a {callable_type}")
            }
        }
        .unwrap();

        let Some(user_component_id) = user_component_id else {
            continue;
        };
        let location = component_db
            .user_component_db()
            .get_location(user_component_id);
        let Some(s) = diagnostics.source(&location) else {
            continue;
        };
        let callable_type = component_db.user_component_db()[user_component_id].kind();
        let s = diagnostic::f_macro_span(s.source(), location)
            .labeled(format!("One of the consuming {callable_type}s"))
            .attach(s);
        consuming_snippets.push(HelpWithSnippet::new(String::new(), s));
    }
    writeln!(
        &mut error_msg,
        "Since I'm not allowed to clone `{type_:?}`, I can't resolve this conflict."
    )
    .unwrap();

    let mut diagnostic = CompilerDiagnostic::builder(anyhow::anyhow!(error_msg));

    if let Some(component_id) = consumed_component_id {
        if let Some(user_component_id) = component_db.user_component_id(component_id) {
            let help_msg = format!(
                "Allow me to clone `{type_:?}` in order to satisfy the borrow checker.\n\
                You can do so by invoking `.cloning(CloningStrategy::CloneIfNecessary)` on the type returned by `.constructor`.",
            );
            let location = component_db
                .user_component_db()
                .get_location(user_component_id);
            let callable_type = component_db.user_component_db()[user_component_id].kind();
            let help = match diagnostics.source(location) {
                None => HelpWithSnippet::new(help_msg, AnnotatedSource::empty()),
                Some(s) => {
                    let s = diagnostic::f_macro_span(s.source(), location)
                        .labeled(format!("The {callable_type} was registered here"))
                        .attach(s);
                    HelpWithSnippet::new(help_msg, s).normalize()
                }
            };
            diagnostic = diagnostic.help_with_snippet(help);
        }
    }

    let help = HelpWithSnippet::new(
        format!(
            "Considering changing the signature of the components that consume `{type_:?}` by value.\n\
            Would a shared reference, `&{type_:?}`, be enough?",
        ),
        AnnotatedSource::empty(),
    );
    diagnostic = diagnostic.help_with_snippet(help);
    for snippet in consuming_snippets {
        diagnostic = diagnostic.help_with_snippet(snippet);
    }

    let diagnostic = suggest_wrapping_in_a_smart_pointer(
        consumed_component_id,
        component_db,
        computation_db,
        diagnostic,
    );

    diagnostics.push(diagnostic.build());
}
