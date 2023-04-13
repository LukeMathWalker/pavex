use guppy::graph::PackageGraph;
use miette::NamedSource;
use petgraph::prelude::EdgeRef;
use petgraph::stable_graph::NodeIndex;

use crate::compiler::analyses::call_graph::borrow_checker::clone::get_clone_component_id;
use crate::compiler::analyses::call_graph::core_graph::RawCallGraph;
use crate::compiler::analyses::call_graph::{
    CallGraph, CallGraphEdgeMetadata, CallGraphNode, NumberOfAllowedInvocations,
};
use crate::compiler::analyses::components::ComponentDb;
use crate::compiler::analyses::computations::ComputationDb;
use crate::diagnostic;
use crate::diagnostic::{
    AnnotatedSnippet, CompilerDiagnostic, HelpWithSnippet, LocationExt, OptionalSourceSpanExt,
};
use crate::rustdoc::CrateCollection;

/// Scan the call graph for a specific kind of borrow-checking violation: node `A` is consumed
/// by value by two or more nodes.
///
/// If this happens, we try to clone `A` (if its cloning strategy allows for it), otherwise we emit
/// an error.
pub(super) fn multiple_consumers(
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

    let indices = call_graph.node_indices().collect::<Vec<_>>();
    for node_id in indices {
        if let CallGraphNode::MatchBranching = call_graph[node_id] {
            continue;
        }

        let consumer_ids: Vec<_> = call_graph
            .edges_directed(node_id, petgraph::Direction::Outgoing)
            .filter_map(|edge| {
                if *edge.weight() == CallGraphEdgeMetadata::Move {
                    Some(edge.target().to_owned())
                } else {
                    None
                }
            })
            .collect();
        if consumer_ids.len() > 1 {
            let CallGraphNode::Compute { component_id, .. } =
                call_graph[node_id].clone() else { continue; };
            let Some(clone_component_id) = get_clone_component_id(
                &component_id,
                package_graph,
                krate_collection,
                component_db,
                computation_db,
            ) else {
                emit_multiple_consumers_error(
                    node_id,
                    consumer_ids,
                    computation_db,
                    component_db,
                    package_graph,
                    &call_graph,
                    diagnostics,
                );
                continue;
            };

            // We only need to clone N-1 times, because the last consumer can
            // simply move the value.
            let (_last, other_ids) = consumer_ids.split_last().unwrap();
            for consumer_id in other_ids {
                let edge_id = call_graph.find_edge(node_id, *consumer_id).unwrap();

                let clone_node_id = call_graph.add_node(CallGraphNode::Compute {
                    component_id: clone_component_id,
                    n_allowed_invocations: NumberOfAllowedInvocations::One,
                });

                // `Clone`'s signature is `fn clone(&self) -> Self`, therefore
                // we must introduce the new node with a "SharedBorrow" edge.
                call_graph.update_edge(node_id, clone_node_id, CallGraphEdgeMetadata::SharedBorrow);
                call_graph.update_edge(clone_node_id, *consumer_id, CallGraphEdgeMetadata::Move);
                call_graph.remove_edge(edge_id);
            }
        }
    }
    CallGraph {
        call_graph,
        root_node_index,
    }
}

fn emit_multiple_consumers_error(
    consumed_node_id: NodeIndex,
    consuming_node_ids: Vec<NodeIndex>,
    computation_db: &ComputationDb,
    component_db: &ComponentDb,
    package_graph: &PackageGraph,
    call_graph: &RawCallGraph,
    diagnostics: &mut Vec<miette::Error>,
) {
    let (consumed_component_id, type_) = match &call_graph[consumed_node_id] {
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

    let n_consumers = consuming_node_ids.len();
    let error_msg = format!(
        "I can't generate code that will pass the borrow checker *and* match the instructions \
        in your blueprint.\n\
        There are {n_consumers} that take `{type_:?}` as an input parameter, consuming it by value. \
        Since I cannot clone `{type_:?}`, I can't resolve this conflict."
    );
    let dummy_source = NamedSource::new("", "");
    let mut diagnostic = CompilerDiagnostic::builder(dummy_source, anyhow::anyhow!(error_msg));

    if let Some(component_id) = consumed_component_id {
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
            "Considering changing the signature of the constructors that consume `{type_:?}` by value.\n\
            Would a shared reference, `&{type_:?}`, be enough?",
        ),
        AnnotatedSnippet::new_with_labels(NamedSource::new("", ""), vec![]),
    );
    diagnostic = diagnostic.help_with_snippet(help);
    for consumer_node_id in consuming_node_ids {
        let CallGraphNode::Compute { component_id, .. } = call_graph[consumer_node_id] else { continue; };
        let Some(user_component_id) = component_db.user_component_id(component_id) else { continue; };
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
        if let Some(source) = source {
            let labeled_span = diagnostic::get_f_macro_invocation_span(&source, location)
                .labeled("One of the consuming constructors".into());
            diagnostic = diagnostic.help_with_snippet(HelpWithSnippet::new(
                String::new(),
                AnnotatedSnippet::new_optional(source, labeled_span),
            ));
        }
    }

    let ref_counting_help = format!("If `{type_:?}` itself cannot implement `Clone`, consider wrapping it in an `std::sync::Rc` or `std::sync::Arc`.");
    let ref_counting_help = HelpWithSnippet::new(
        ref_counting_help,
        AnnotatedSnippet::new_with_labels(NamedSource::new("", ""), vec![]),
    );
    diagnostic = diagnostic.help_with_snippet(ref_counting_help);

    diagnostics.push(diagnostic.build().into());
}
