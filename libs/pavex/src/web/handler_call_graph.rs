use std::collections::HashMap;

use bimap::BiHashMap;
use guppy::PackageId;
use indexmap::{IndexMap, IndexSet};
use petgraph::prelude::{DfsPostOrder, StableDiGraph};
use petgraph::stable_graph::NodeIndex;
use petgraph::visit::Reversed;
use petgraph::Direction;
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::ItemFn;

use pavex_builder::Lifecycle;

use crate::language::{Callable, ResolvedType};
use crate::web::codegen_utils;
use crate::web::codegen_utils::{Fragment, VariableNameGenerator};
use crate::web::dependency_graph::{CallableDependencyGraph, DependencyGraphNode};

/// The handler dependency graph ([`CallableDependencyGraph`]) is focused on data - it tells us
/// what types are needed to build the input parameters for a certain handler.
///
/// This is not enough to perform code generation - we also need to know what is the _lifecycle_
/// for each of those types.  
/// E.g. singletons should be constructed once and re-used throughout the entire lifetime of the
/// application; this implies that the generated code for handling a single request should not
/// call the singleton constructor - it should fetch it from the server state!
///
/// The handler call graph tries to capture this information.  
/// In the dependency graph, each type appears exactly once, no matter how many times it's required
/// as input for other constructors.
/// In the call graph, each type appears as many times as it needs to be constructed - either
/// by calling the constructor or receiving it as input.
/// Singletons and request-scoped types will appear only once. Transient types will appear
/// as many times as they are used.
#[derive(Debug)]
pub(crate) struct HandlerCallGraph {
    pub(crate) call_graph: StableDiGraph<DependencyGraphNode, ()>,
    pub(crate) handler_node_index: NodeIndex,
    pub(crate) lifecycles: HashMap<ResolvedType, Lifecycle>,
    pub(crate) constructors: IndexMap<ResolvedType, Callable>,
    pub(crate) input_parameter_types: IndexSet<ResolvedType>,
}

impl HandlerCallGraph {
    pub(crate) fn new(
        dependency_graph: &'_ CallableDependencyGraph,
        lifecycles: HashMap<ResolvedType, Lifecycle>,
        constructors: IndexMap<ResolvedType, Callable>,
    ) -> Self {
        // Vec<(index in dependency graph, parent index in call graph)>
        let CallableDependencyGraph {
            dependency_graph,
            callable_node_index: handler_node_index,
        } = dependency_graph;
        let mut nodes_to_be_visited = vec![(*handler_node_index, None)];
        let mut scoped_or_longer_indexes = HashMap::<u32, NodeIndex>::new();
        let mut call_graph = StableDiGraph::new();
        while let Some((dep_node_index, call_parent_node_index)) = nodes_to_be_visited.pop() {
            let node = &dependency_graph[dep_node_index];
            // Determine how many times the constructor for this type should be invoked in the call graph.
            // If we are dealing with a singleton, we need to make sure it's invoked only once.
            // Transient components, instead, appear as many times as they are used in the call graph.
            // We treat compute nodes as singletons as well.
            let call_node_index = match node {
                DependencyGraphNode::Compute(_) => {
                    let index = call_graph.add_node(node.to_owned());
                    scoped_or_longer_indexes.insert(dep_node_index, index);
                    index
                }
                DependencyGraphNode::Type(t) => {
                    let lifecycle = lifecycles
                        .get(t)
                        .cloned()
                        .unwrap_or(Lifecycle::RequestScoped);
                    match lifecycle {
                        Lifecycle::Singleton | Lifecycle::RequestScoped => scoped_or_longer_indexes
                            .get(&dep_node_index)
                            .cloned()
                            .unwrap_or_else(|| {
                                let index = call_graph.add_node(node.to_owned());
                                scoped_or_longer_indexes.insert(dep_node_index, index);
                                index
                            }),
                        Lifecycle::Transient => call_graph.add_node(node.to_owned()),
                    }
                }
            };
            if let Some(call_parent_node_index) = call_parent_node_index {
                call_graph.add_edge(call_node_index, call_parent_node_index, ());
            }

            // Singleton types are constructed in the initialization phase of the server, we do
            // not build them every single time we receive a request.
            // Therefore we should not account for their dependencies (and the necessary constructor
            // calls) when building the call graph for a request handler.
            if let DependencyGraphNode::Type(t) = node {
                if Some(&Lifecycle::Singleton) == lifecycles.get(t) {
                    continue;
                }
            }

            let dependencies_node_indexes = dependency_graph
                .graph
                .neighbors_directed(dep_node_index, Direction::Incoming);
            for dependency_node_index in dependencies_node_indexes {
                nodes_to_be_visited.push((dependency_node_index, Some(call_node_index)));
            }
        }
        let input_parameter_types = required_inputs(&call_graph, &lifecycles, &constructors);
        HandlerCallGraph {
            call_graph,
            handler_node_index: scoped_or_longer_indexes[handler_node_index],
            lifecycles,
            constructors,
            input_parameter_types,
        }
    }

    pub fn dot(&self, package_ids2names: &BiHashMap<&'_ PackageId, String>) -> String {
        let config = [
            petgraph::dot::Config::EdgeNoLabel,
            petgraph::dot::Config::NodeNoLabel,
        ];
        format!(
            "{:?}",
            petgraph::dot::Dot::with_attr_getters(
                &self.call_graph,
                &config,
                &|_, _| "".to_string(),
                &|_, (_, node)| {
                    match node {
                        DependencyGraphNode::Compute(c) => {
                            format!("label = \"{}\"", c.render_signature(package_ids2names))
                        }
                        DependencyGraphNode::Type(t) => {
                            format!("label = \"{}\"", t.render_type(package_ids2names))
                        }
                    }
                },
            )
        )
    }
}

/// Return the set of types that must be provided as input to build the handler's input parameters
/// and invoke it.
///
/// In other words, return the set of types that either:
/// - do not have a registered constructor;
/// - have `Singleton` as lifecycle.
///
/// We return a `IndexSet` instead of a `HashSet` because we want a consistent ordering for the input
/// parameters - it will be used in other parts of the crate to provide instances of those types
/// in the expected order.
fn required_inputs(
    call_graph: &StableDiGraph<DependencyGraphNode, ()>,
    lifecycles: &HashMap<ResolvedType, Lifecycle>,
    constructors: &IndexMap<ResolvedType, Callable>,
) -> IndexSet<ResolvedType> {
    call_graph
        .node_weights()
        .filter_map(|node| {
            if let DependencyGraphNode::Type(type_) = node {
                if !constructors.contains_key(type_)
                    || lifecycles.get(type_) == Some(&Lifecycle::Singleton)
                {
                    return Some(type_);
                }
            }
            None
        })
        .cloned()
        .collect()
}

pub(crate) fn codegen<'a>(
    graph: &HandlerCallGraph,
    package_id2name: &BiHashMap<&'a PackageId, String>,
) -> Result<ItemFn, anyhow::Error> {
    let HandlerCallGraph {
        call_graph,
        handler_node_index,
        lifecycles,
        constructors,
        input_parameter_types,
    } = graph;
    let mut dfs = DfsPostOrder::new(Reversed(call_graph), *handler_node_index);

    let mut parameter_bindings = HashMap::new();
    let mut variable_generator = VariableNameGenerator::default();

    let mut scoped_constructors = IndexMap::<NodeIndex, TokenStream>::new();
    let mut blocks = HashMap::<NodeIndex, Fragment>::new();

    while let Some(node_index) = dfs.next(Reversed(call_graph)) {
        let node = &call_graph[node_index];
        match node {
            DependencyGraphNode::Type(t) => {
                let lifecycle = lifecycles
                    .get(t)
                    .cloned()
                    .unwrap_or(Lifecycle::RequestScoped);
                match lifecycle {
                    Lifecycle::Singleton => {
                        let parameter_name = variable_generator.generate();
                        parameter_bindings.insert(t.to_owned(), parameter_name.clone());
                        blocks.insert(node_index, Fragment::VariableReference(parameter_name));
                    }
                    Lifecycle::Transient => {
                        let callable = &constructors[t];
                        let block = codegen_utils::codegen_call_block(
                            call_graph,
                            callable,
                            node_index,
                            &mut blocks,
                            &mut variable_generator,
                            package_id2name,
                        )?;
                        blocks.insert(node_index, block);
                    }
                    Lifecycle::RequestScoped => {
                        let parameter_name = variable_generator.generate();
                        match constructors.get(t) {
                            None => {
                                parameter_bindings.insert(t.to_owned(), parameter_name.clone());
                            }
                            Some(callable) => {
                                let block = codegen_utils::codegen_call_block(
                                    call_graph,
                                    callable,
                                    node_index,
                                    &mut blocks,
                                    &mut variable_generator,
                                    package_id2name,
                                )?;
                                let block = quote! {
                                    let #parameter_name = #block;
                                };
                                scoped_constructors.insert(node_index, block);
                            }
                        };
                        blocks.insert(node_index, Fragment::VariableReference(parameter_name));
                    }
                }
            }
            DependencyGraphNode::Compute(callable) => {
                let block = codegen_utils::codegen_call_block(
                    call_graph,
                    callable,
                    node_index,
                    &mut blocks,
                    &mut variable_generator,
                    package_id2name,
                )?;
                blocks.insert(node_index, block);
            }
        }
    }

    let handler = match &call_graph[*handler_node_index] {
        DependencyGraphNode::Compute(c) => c,
        DependencyGraphNode::Type(_) => unreachable!(),
    };
    let code = match blocks.remove(handler_node_index) {
        None => unreachable!(),
        Some(b) => {
            let inputs = input_parameter_types.iter().map(|type_| {
                let variable_name = &parameter_bindings[type_];
                let variable_type = type_.syn_type(package_id2name);
                quote! { #variable_name: #variable_type }
            });
            let output_type = handler.output_fq_path.syn_type(package_id2name);
            let scoped_constructors = scoped_constructors.values();
            let b = match b {
                Fragment::VariableReference(_) => unreachable!(),
                Fragment::Block(b) => {
                    let stmts = b.stmts.iter();
                    quote! {
                        #(#stmts)*
                    }
                }
                Fragment::Statement(s) => s.to_token_stream(),
            };
            syn::parse2(quote! {
                pub fn handler(#(#inputs),*) -> #output_type {
                    #(#scoped_constructors)*
                    #b
                }
            })
            .unwrap()
        }
    };
    Ok(code)
}
