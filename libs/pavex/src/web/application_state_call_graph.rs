use std::collections::HashMap;

use bimap::BiHashMap;
use guppy::PackageId;
use indexmap::IndexMap;
use petgraph::prelude::StableDiGraph;
use petgraph::stable_graph::NodeIndex;
use petgraph::Direction;
use proc_macro2::Ident;

use pavex_builder::Lifecycle;

use crate::language::{Callable, InvocationStyle, ResolvedPath, ResolvedPathSegment, ResolvedType};
use crate::web::app::GENERATED_APP_PACKAGE_ID;
use crate::web::constructors::Constructor;
use crate::web::dependency_graph::{CallableDependencyGraph, DependencyGraphNode};
use crate::web::handler_call_graph::{
    CallGraph, HandlerCallGraphNode, NumberOfAllowedInvocations, VisitorStackElement,
};

#[tracing::instrument(name = "compute_application_state_call_graph", skip_all)]
pub(crate) fn application_state_call_graph(
    runtime_singleton_bindings: &BiHashMap<Ident, ResolvedType>,
    lifecycles: &HashMap<ResolvedType, Lifecycle>,
    constructors: IndexMap<ResolvedType, Constructor>,
) -> CallGraph {
    // We build a "mock" callable that has the right inputs in order to drive the machinery
    // that builds the dependency graph.
    let application_state_constructor = Callable {
        is_async: false,
        output: ResolvedType {
            package_id: PackageId::new(GENERATED_APP_PACKAGE_ID),
            base_type: vec!["crate".into(), "ApplicationState".into()],
            generic_arguments: vec![],
            is_shared_reference: false,
        },
        path: ResolvedPath {
            segments: vec![
                ResolvedPathSegment {
                    ident: "crate".into(),
                    generic_arguments: vec![],
                },
                ResolvedPathSegment {
                    ident: "ApplicationState".into(),
                    generic_arguments: vec![],
                },
            ],
            package_id: PackageId::new(GENERATED_APP_PACKAGE_ID),
        },
        inputs: runtime_singleton_bindings.right_values().cloned().collect(),
        invocation_style: InvocationStyle::StructLiteral {
            field_names: runtime_singleton_bindings
                .iter()
                .map(|(ident, type_)| (ident.to_string(), type_.to_owned()))
                .collect(),
        },
    };
    let CallableDependencyGraph {
        dependency_graph,
        callable_node_index,
    } = CallableDependencyGraph::new(application_state_constructor, &constructors);

    let mut nodes_to_be_visited = vec![VisitorStackElement::orphan(callable_node_index)];
    // HashMap<index in dependency graph, index in call graph>
    let mut singleton_or_longer_indexes = HashMap::<u32, NodeIndex>::new();
    let mut call_graph = StableDiGraph::new();
    while let Some(node_to_be_visited) = nodes_to_be_visited.pop() {
        let (dep_node_index, call_parent_node_index) = (
            node_to_be_visited.dependency_graph_index,
            node_to_be_visited.call_graph_parent_index,
        );
        let node = &dependency_graph[dep_node_index];
        let call_node_index = {
            let call_graph_node = match node {
                DependencyGraphNode::Compute(c) => HandlerCallGraphNode::Compute {
                    constructor: Constructor::Callable(c.to_owned()),
                    n_allowed_invocations: NumberOfAllowedInvocations::One,
                },
                DependencyGraphNode::Type(t) => match lifecycles.get(t).cloned() {
                    Some(Lifecycle::Singleton) => HandlerCallGraphNode::Compute {
                        constructor: constructors[t].to_owned(),
                        n_allowed_invocations: NumberOfAllowedInvocations::One,
                    },
                    None => HandlerCallGraphNode::InputParameter(t.to_owned()),
                    Some(Lifecycle::RequestScoped) => {
                        panic!("Singletons should not depend on types with a request-scoped lifecycle.")
                    }
                    Some(Lifecycle::Transient) => {
                        panic!("Singletons should not depend on types with a transient lifecycle.")
                    }
                },
            };
            match call_graph_node {
                HandlerCallGraphNode::Compute {
                    n_allowed_invocations,
                    ..
                } => match n_allowed_invocations {
                    NumberOfAllowedInvocations::One => singleton_or_longer_indexes
                        .get(&dep_node_index)
                        .cloned()
                        .unwrap_or_else(|| {
                            let index = call_graph.add_node(call_graph_node);
                            singleton_or_longer_indexes.insert(dep_node_index, index);
                            index
                        }),
                    NumberOfAllowedInvocations::Multiple => call_graph.add_node(call_graph_node),
                },
                HandlerCallGraphNode::InputParameter(_) => singleton_or_longer_indexes
                    .get(&dep_node_index)
                    .cloned()
                    .unwrap_or_else(|| {
                        let index = call_graph.add_node(call_graph_node);
                        singleton_or_longer_indexes.insert(dep_node_index, index);
                        index
                    }),
            }
        };

        if let Some(call_parent_node_index) = call_parent_node_index {
            call_graph.add_edge(call_node_index, call_parent_node_index, ());
        }

        // We need to recursively build the input types for all our constructors;
        if let HandlerCallGraphNode::Compute { .. } = call_graph[call_node_index] {
            let dependencies_node_indexes = dependency_graph
                .graph
                .neighbors_directed(dep_node_index, Direction::Incoming);
            for dependency_node_index in dependencies_node_indexes {
                nodes_to_be_visited.push(VisitorStackElement {
                    dependency_graph_index: dependency_node_index,
                    call_graph_parent_index: Some(call_node_index),
                });
            }
        }
    }
    CallGraph {
        call_graph,
        handler_node_index: singleton_or_longer_indexes[&callable_node_index],
    }
}
