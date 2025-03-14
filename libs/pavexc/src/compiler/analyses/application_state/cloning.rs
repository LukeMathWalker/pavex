use indexmap::IndexSet;
use pavex_bp_schema::{CloningStrategy, Lifecycle};
use petgraph::visit::EdgeRef;

use crate::{
    compiler::{
        analyses::{
            call_graph::{CallGraphEdgeMetadata, CallGraphNode, InputParameterSource},
            components::{ComponentDb, ComponentId},
            computations::ComputationDb,
            processing_pipeline::RequestHandlerPipeline,
        },
        computation::Computation,
        traits::assert_trait_is_implemented,
        utils::process_framework_path,
    },
    diagnostic::{
        self, AnnotatedSource, CompilerDiagnostic, HelpWithSnippet, OptionalLabeledSpanExt,
        OptionalSourceSpanExt,
    },
    language::ResolvedType,
    rustdoc::CrateCollection,
};

/// Verify that all singletons that need to be cloned at runtime can actually be cloned:
/// - The `Clone` trait is implemented
/// - Their cloning strategy permits cloning
#[tracing::instrument(
    name = "Verify cloning strategy for singletons used at runtime",
    skip_all
)]
pub(crate) fn runtime_singletons_can_be_cloned_if_needed<'a>(
    handler_pipelines: impl Iterator<Item = &'a RequestHandlerPipeline>,
    component_db: &ComponentDb,
    computation_db: &ComputationDb,
    krate_collection: &CrateCollection,
    diagnostics: &mut crate::diagnostic::DiagnosticSink,
) {
    let copy = process_framework_path("core::marker::Copy", krate_collection);
    let ResolvedType::ResolvedPath(copy) = copy else {
        unreachable!()
    };
    let clone = process_framework_path("core::clone::Clone", krate_collection);
    let ResolvedType::ResolvedPath(clone) = clone else {
        unreachable!()
    };

    for pipeline in handler_pipelines {
        for graph in pipeline.graph_iter() {
            let owned_singleton_inputs: IndexSet<_> = graph
                .call_graph
                .node_indices()
                .filter_map(|node_index| {
                    let node = &graph.call_graph[node_index];
                    match node {
                        CallGraphNode::Compute { .. } | CallGraphNode::MatchBranching => None,
                        CallGraphNode::InputParameter { type_, source } => {
                            match type_ {
                                ResolvedType::ScalarPrimitive(_)
                                | ResolvedType::Reference(_)
                                | ResolvedType::Slice(_) => {
                                    return None;
                                }
                                ResolvedType::ResolvedPath(_) | ResolvedType::Tuple(_) => {}
                                ResolvedType::Generic(_) => unreachable!(),
                            };
                            let InputParameterSource::Component(id) = source else {
                                return None;
                            };
                            if component_db.lifecycle(*id) != Lifecycle::Singleton {
                                return None;
                            }
                            Some((node_index, type_, *id))
                        }
                    }
                })
                .collect();
            for (node_index, type_, id) in owned_singleton_inputs {
                if assert_trait_is_implemented(krate_collection, type_, &copy).is_ok() {
                    continue;
                }
                if component_db.cloning_strategy(id) == CloningStrategy::CloneIfNecessary {
                    continue;
                }
                let is_clone = assert_trait_is_implemented(krate_collection, type_, &clone).is_ok();
                for edge in graph
                    .call_graph
                    .edges_directed(node_index, petgraph::Direction::Outgoing)
                {
                    if &CallGraphEdgeMetadata::Move != edge.weight() {
                        continue;
                    }
                    let consumer = &graph.call_graph[edge.target()];
                    let CallGraphNode::Compute {
                        component_id: consumer_id,
                        ..
                    } = consumer
                    else {
                        continue;
                    };
                    must_be_clonable(
                        type_,
                        is_clone,
                        id,
                        *consumer_id,
                        component_db,
                        computation_db,
                        diagnostics,
                    );
                }
            }
        }
    }
}

fn must_be_clonable(
    type_: &ResolvedType,
    is_clone: bool,
    component_id: ComponentId,
    consumer_id: ComponentId,
    component_db: &ComponentDb,
    computation_db: &ComputationDb,
    diagnostics: &mut crate::diagnostic::DiagnosticSink,
) {
    let component_id = component_db
        .derived_from(&component_id)
        .unwrap_or(component_id);
    if component_db.user_component_id(consumer_id).is_none()
        && component_db.derived_from(&consumer_id).is_none()
    {
        return;
    }
    let consumer = component_db.hydrated_component(consumer_id, computation_db);
    let Computation::Callable(consumer_callable) = consumer.computation() else {
        return;
    };

    let user_component_id = component_db.user_component_id(component_id).unwrap();
    let user_component_db = &component_db.user_component_db();
    let location = user_component_db.get_location(user_component_id);
    let source = diagnostics.source(&location).map(|s| {
        diagnostic::f_macro_span(s.source(), location)
            .labeled("It was registered here".to_string())
            .attach(s)
    });
    let e = anyhow::anyhow!(
        "I can't generate code that will pass the borrow checker *and* match the \
        instructions in your blueprint.\n\
        `{}` consumes `{}` by value, but `{}` is a singleton and can't be moved out of `ApplicationState`.\n\
        Cloning `{}` would fix this, but its cloning strategy is set to `NeverClone`.",
        consumer_callable.path,
        type_.display_for_error(),
        type_.display_for_error(),
        type_.display_for_error(),
    );
    let help = is_clone.then(|| {
        format!(
            "Set the cloning strategy for `{}` to `CloneIfNecessary`.",
            type_.display_for_error()
        )
    });
    let second_help = format!(
        "Can `{}` take a reference to `{}`, rather than consuming it by value?",
        consumer_callable.path,
        type_.display_for_error()
    );
    let diagnostic = CompilerDiagnostic::builder(e)
        .optional_source(source)
        .optional_help(help)
        .help_with_snippet(HelpWithSnippet::new(second_help, AnnotatedSource::empty()))
        .build();
    diagnostics.push(diagnostic);
}
