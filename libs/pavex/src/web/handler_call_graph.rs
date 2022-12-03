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

use crate::language::ResolvedType;
use crate::web::codegen_utils;
use crate::web::codegen_utils::{Fragment, VariableNameGenerator};
use crate::web::constructors::Constructor;
use crate::web::dependency_graph::{CallableDependencyGraph, DependencyGraphNode};

/// The handler dependency graph ([`CallableDependencyGraph`]) is focused on data - it tells us
/// what types are needed to build the input parameters for a certain handler.
///
/// This is not enough to perform code generation - we also need to know the _lifecycle_
/// of each of those types.
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
    pub(crate) call_graph: StableDiGraph<HandlerCallGraphNode, ()>,
    pub(crate) handler_node_index: NodeIndex,
    pub(crate) lifecycles: HashMap<ResolvedType, Lifecycle>,
    pub(crate) input_parameter_types: IndexSet<ResolvedType>,
}

#[derive(Clone, Debug)]
pub(crate) enum HandlerCallGraphNode {
    Compute(Constructor),
    InputParameter(ResolvedType),
}

pub enum NumberOfAllowedInvocations {
    One,
    Multiple,
}

struct VisitorStackElement {
    dependency_graph_index: u32,
    call_graph_parent_index: Option<NodeIndex>,
}

impl VisitorStackElement {
    /// A short-cut to add a node without a parent to the visitor stack.
    pub fn orphan(dependency_graph_index: u32) -> Self {
        Self {
            dependency_graph_index,
            call_graph_parent_index: None,
        }
    }
}

impl HandlerCallGraph {
    #[tracing::instrument(name = "compute_handler_call_graph", skip_all)]
    pub(crate) fn new(
        dependency_graph: &'_ CallableDependencyGraph,
        mut lifecycles: HashMap<ResolvedType, Lifecycle>,
        constructors: IndexMap<ResolvedType, Constructor>,
    ) -> Self {
        let CallableDependencyGraph {
            dependency_graph,
            callable_node_index: handler_node_index,
        } = dependency_graph;
        let mut nodes_to_be_visited = vec![VisitorStackElement::orphan(*handler_node_index)];
        // HashMap<index in dependency graph, index in call graph>
        let mut scoped_or_longer_indexes = HashMap::<u32, NodeIndex>::new();
        let mut call_graph = StableDiGraph::<HandlerCallGraphNode, ()>::new();
        while let Some(node_to_be_visited) = nodes_to_be_visited.pop() {
            let (dep_node_index, call_parent_node_index) = (
                node_to_be_visited.dependency_graph_index,
                node_to_be_visited.call_graph_parent_index,
            );
            let node = &dependency_graph[dep_node_index];
            // Determine how many times the constructor for this type should be invoked in the call graph.
            // If we are dealing with a singleton, we need to make sure it's invoked only once.
            // Transient components, instead, appear as many times as they are used in the call graph.
            // We treat compute nodes as singletons as well.
            let call_node_index = {
                let (n_allowed_invocations, new_node) = match node {
                    DependencyGraphNode::Compute(c) => {
                        let node =
                            HandlerCallGraphNode::Compute(Constructor::Callable(c.to_owned()));
                        lifecycles.insert(c.output.to_owned(), Lifecycle::RequestScoped);
                        (NumberOfAllowedInvocations::One, node)
                    }
                    DependencyGraphNode::Type(t) => match lifecycles.get(t).cloned() {
                        Some(Lifecycle::Singleton) | None => (
                            NumberOfAllowedInvocations::One,
                            HandlerCallGraphNode::InputParameter(t.to_owned()),
                        ),
                        Some(Lifecycle::RequestScoped) => (
                            NumberOfAllowedInvocations::One,
                            HandlerCallGraphNode::Compute(constructors[t].to_owned()),
                        ),
                        Some(Lifecycle::Transient) => (
                            NumberOfAllowedInvocations::Multiple,
                            HandlerCallGraphNode::Compute(constructors[t].to_owned()),
                        ),
                    },
                };
                match n_allowed_invocations {
                    NumberOfAllowedInvocations::One => scoped_or_longer_indexes
                        .get(&dep_node_index)
                        .cloned()
                        .unwrap_or_else(|| {
                            let index = call_graph.add_node(new_node);
                            scoped_or_longer_indexes.insert(dep_node_index, index);
                            index
                        }),
                    NumberOfAllowedInvocations::Multiple => call_graph.add_node(new_node),
                }
            };

            if let Some(call_parent_node_index) = call_parent_node_index {
                call_graph.add_edge(call_node_index, call_parent_node_index, ());
            }

            // We need to recursively build the input types for all our constructors;
            if let HandlerCallGraphNode::Compute(_) = call_graph[call_node_index] {
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
        let input_parameter_types = required_inputs(&call_graph);
        HandlerCallGraph {
            call_graph,
            handler_node_index: scoped_or_longer_indexes[handler_node_index],
            lifecycles,
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
                        HandlerCallGraphNode::Compute(c) => match c {
                            Constructor::BorrowSharedReference(r) => {
                                format!("label = \"&{}\"", r.input.render_type(package_ids2names))
                            }
                            Constructor::Callable(c) => {
                                format!("label = \"{}\"", c.render_signature(package_ids2names))
                            }
                        },
                        HandlerCallGraphNode::InputParameter(t) => {
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
/// - have `Singleton` as lifecycle;
/// - are references to a `Singleton`.
///
/// We return a `IndexSet` instead of a `HashSet` because we want a consistent ordering for the input
/// parameters - it will be used in other parts of the crate to provide instances of those types
/// in the expected order.
fn required_inputs(call_graph: &StableDiGraph<HandlerCallGraphNode, ()>) -> IndexSet<ResolvedType> {
    let singletons_or_missing_constructors: IndexSet<ResolvedType> = call_graph
        .node_weights()
        .filter_map(|node| match node {
            HandlerCallGraphNode::Compute(_) => None,
            HandlerCallGraphNode::InputParameter(i) => Some(i),
        })
        .cloned()
        .collect();
    singletons_or_missing_constructors
}

pub(crate) fn codegen<'a>(
    graph: &HandlerCallGraph,
    package_id2name: &BiHashMap<&'a PackageId, String>,
) -> Result<ItemFn, anyhow::Error> {
    let HandlerCallGraph {
        call_graph,
        handler_node_index,
        lifecycles,
        input_parameter_types,
    } = graph;
    let mut dfs = DfsPostOrder::new(Reversed(call_graph), *handler_node_index);

    let mut parameter_bindings: HashMap<ResolvedType, syn::Ident> = HashMap::new();
    let mut variable_generator = VariableNameGenerator::default();

    let mut scoped_constructors = IndexMap::<NodeIndex, TokenStream>::new();
    let mut blocks = HashMap::<NodeIndex, Fragment>::new();

    while let Some(node_index) = dfs.next(Reversed(call_graph)) {
        let node = &call_graph[node_index];
        match node {
            HandlerCallGraphNode::Compute(constructor) => {
                let lifecycle = lifecycles[constructor.output_type()].to_owned();
                match lifecycle {
                    Lifecycle::Singleton => match constructor {
                        Constructor::BorrowSharedReference(s) => {
                            if let Some(parameter_name) = parameter_bindings.get(&s.input) {
                                blocks.insert(
                                    node_index,
                                    Fragment::BorrowSharedReference(parameter_name.to_owned()),
                                );
                            } else {
                                let parameter_name = variable_generator.generate();
                                parameter_bindings.insert(
                                    constructor.output_type().to_owned(),
                                    parameter_name.clone(),
                                );
                                blocks.insert(
                                    node_index,
                                    Fragment::VariableReference(parameter_name),
                                );
                            }
                        }
                        Constructor::Callable(_) => {
                            let parameter_name = variable_generator.generate();
                            parameter_bindings.insert(
                                constructor.output_type().to_owned(),
                                parameter_name.clone(),
                            );
                            blocks.insert(node_index, Fragment::VariableReference(parameter_name));
                        }
                    },
                    Lifecycle::Transient => match constructor {
                        Constructor::Callable(callable) => {
                            let block = codegen_utils::_codegen_call_block(
                                get_node_type_inputs(node_index, call_graph),
                                callable,
                                &mut blocks,
                                &mut variable_generator,
                                package_id2name,
                            )?;
                            blocks.insert(node_index, block);
                        }
                        Constructor::BorrowSharedReference(_) => {
                            let dependencies =
                                call_graph.neighbors_directed(node_index, Direction::Incoming);
                            let dependency_indexes: Vec<_> = dependencies.collect();
                            assert_eq!(1, dependency_indexes.len());
                            let dependency_index = dependency_indexes.first().unwrap();
                            match &blocks[dependency_index] {
                                Fragment::VariableReference(binding_name) => {
                                    blocks.insert(
                                        node_index,
                                        Fragment::BorrowSharedReference(binding_name.to_owned()),
                                    );
                                }
                                Fragment::Block(b) => {
                                    blocks.insert(
                                        node_index,
                                        Fragment::Block(
                                            syn::parse2(quote! {
                                                &#b
                                            })
                                            .unwrap(),
                                        ),
                                    );
                                }
                                Fragment::Statement(b) => {
                                    blocks.insert(
                                        node_index,
                                        Fragment::Statement(
                                            syn::parse2(quote! {
                                                &#b;
                                            })
                                            .unwrap(),
                                        ),
                                    );
                                }
                                Fragment::BorrowSharedReference(_) => {
                                    unreachable!()
                                }
                            }
                        }
                    },
                    Lifecycle::RequestScoped => match constructor {
                        Constructor::Callable(callable) => {
                            let block = codegen_utils::_codegen_call_block(
                                get_node_type_inputs(node_index, call_graph),
                                callable,
                                &mut blocks,
                                &mut variable_generator,
                                package_id2name,
                            )?;

                            let has_dependents = call_graph
                                .neighbors_directed(node_index, Direction::Outgoing)
                                .next()
                                .is_some();
                            if !has_dependents {
                                // This is a leaf in the call dependency graph for this handler,
                                // which implies it is a constructor for the return type!
                                // Instead of creating an unnecessary variable binding, we add
                                // the raw expression block - which can then be used as-is for
                                // returning the value.
                                blocks.insert(node_index, block);
                            } else {
                                // We have at least one dependent, this callable does not return
                                // the output type for this handler.
                                // We bind the constructed value to a variable name and instruct
                                // all dependents to refer to the constructed value via that
                                // variable name.
                                let parameter_name = variable_generator.generate();
                                let block = quote! {
                                    let #parameter_name = #block;
                                };
                                scoped_constructors.insert(node_index, block);
                                blocks.insert(
                                    node_index,
                                    Fragment::VariableReference(parameter_name),
                                );
                            }
                        }
                        Constructor::BorrowSharedReference(_) => {
                            let dependencies =
                                call_graph.neighbors_directed(node_index, Direction::Incoming);
                            let dependency_indexes: Vec<_> = dependencies.collect();
                            assert_eq!(1, dependency_indexes.len());
                            let dependency_index = dependency_indexes.first().unwrap();
                            match &blocks[dependency_index] {
                                Fragment::VariableReference(binding_name) => {
                                    blocks.insert(
                                        node_index,
                                        Fragment::BorrowSharedReference(binding_name.to_owned()),
                                    );
                                }
                                Fragment::BorrowSharedReference(_)
                                | Fragment::Statement(_)
                                | Fragment::Block(_) => unreachable!(),
                            }
                        }
                    },
                }
            }
            HandlerCallGraphNode::InputParameter(input_type) => {
                let parameter_name = variable_generator.generate();
                parameter_bindings.insert(input_type.to_owned(), parameter_name.clone());
                blocks.insert(node_index, Fragment::VariableReference(parameter_name));
            }
        }
    }

    let handler = match &call_graph[*handler_node_index] {
        HandlerCallGraphNode::Compute(Constructor::Callable(c)) => c,
        _ => unreachable!(),
    };
    let code = {
        let inputs = input_parameter_types.iter().map(|type_| {
            let variable_name = &parameter_bindings[type_];
            let variable_type = type_.syn_type(package_id2name);
            quote! { #variable_name: #variable_type }
        });
        let output_type = handler.output.syn_type(package_id2name);
        let scoped_constructors = scoped_constructors.values();
        let b = match blocks.remove(handler_node_index).unwrap() {
            Fragment::Block(b) => {
                let s = b.stmts;
                quote! { #(#s)* }
            }
            Fragment::Statement(b) => b.to_token_stream(),
            _ => {
                unreachable!()
            }
        };
        syn::parse2(quote! {
            pub async fn handler(#(#inputs),*) -> #output_type {
                #(#scoped_constructors)*
                #b
            }
        })
        .unwrap()
    };
    Ok(code)
}

fn get_node_type_inputs(
    node_index: NodeIndex,
    call_graph: &StableDiGraph<HandlerCallGraphNode, ()>,
) -> impl Iterator<Item = (NodeIndex, &ResolvedType)> {
    call_graph
        .neighbors_directed(node_index, Direction::Incoming)
        .map(|n| {
            let type_ = match &call_graph[n] {
                HandlerCallGraphNode::Compute(c) => c.output_type(),
                HandlerCallGraphNode::InputParameter(i) => i,
            };
            (n, type_)
        })
}
