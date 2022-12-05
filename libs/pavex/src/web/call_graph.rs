use std::collections::HashMap;

use bimap::BiHashMap;
use guppy::PackageId;
use indexmap::{IndexMap, IndexSet};
use petgraph::prelude::{DfsPostOrder, StableDiGraph};
use petgraph::stable_graph::NodeIndex;
use petgraph::visit::Reversed;
use petgraph::Direction;
use proc_macro2::{Ident, TokenStream};
use quote::{quote, ToTokens};
use syn::ItemFn;

use pavex_builder::Lifecycle;

use crate::language::{Callable, InvocationStyle, ResolvedPath, ResolvedPathSegment, ResolvedType};
use crate::web::app::GENERATED_APP_PACKAGE_ID;
use crate::web::codegen_utils;
use crate::web::codegen_utils::{Fragment, VariableNameGenerator};
use crate::web::constructors::Constructor;
use crate::web::dependency_graph::{CallableDependencyGraph, DependencyGraphNode};

/// Build a [`CallGraph`] for the application state.
#[tracing::instrument(name = "compute_application_state_call_graph", skip_all)]
pub(crate) fn application_state_call_graph(
    runtime_singleton_bindings: &BiHashMap<Ident, ResolvedType>,
    lifecycles: &HashMap<ResolvedType, Lifecycle>,
    constructors: IndexMap<ResolvedType, Constructor>,
) -> CallGraph {
    fn dependency_graph_node2call_graph_node(
        node: &DependencyGraphNode,
        lifecycles: &HashMap<ResolvedType, Lifecycle>,
        constructors: &IndexMap<ResolvedType, Constructor>,
    ) -> HandlerCallGraphNode {
        match node {
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
        }
    }

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
    let dependency_graph =
        CallableDependencyGraph::new(application_state_constructor, &constructors);
    dependency_graph2call_graph(
        &dependency_graph,
        lifecycles,
        &constructors,
        dependency_graph_node2call_graph_node,
    )
}

/// Build a [`CallGraph`] for a request handler.
#[tracing::instrument(name = "compute_handler_call_graph", skip_all)]
pub(crate) fn handler_call_graph(
    dependency_graph: &'_ CallableDependencyGraph,
    lifecycles: &HashMap<ResolvedType, Lifecycle>,
    constructors: &IndexMap<ResolvedType, Constructor>,
) -> CallGraph {
    fn dependency_graph_node2call_graph_node(
        node: &DependencyGraphNode,
        lifecycles: &HashMap<ResolvedType, Lifecycle>,
        constructors: &IndexMap<ResolvedType, Constructor>,
    ) -> HandlerCallGraphNode {
        match node {
            DependencyGraphNode::Compute(c) => HandlerCallGraphNode::Compute {
                constructor: Constructor::Callable(c.to_owned()),
                n_allowed_invocations: NumberOfAllowedInvocations::One,
            },
            DependencyGraphNode::Type(t) => match lifecycles.get(t).cloned() {
                Some(Lifecycle::Singleton) | None => {
                    HandlerCallGraphNode::InputParameter(t.to_owned())
                }
                Some(Lifecycle::RequestScoped) => HandlerCallGraphNode::Compute {
                    constructor: constructors[t].to_owned(),
                    n_allowed_invocations: NumberOfAllowedInvocations::One,
                },
                Some(Lifecycle::Transient) => HandlerCallGraphNode::Compute {
                    constructor: constructors[t].to_owned(),
                    n_allowed_invocations: NumberOfAllowedInvocations::Multiple,
                },
            },
        }
    }
    dependency_graph2call_graph(
        dependency_graph,
        lifecycles,
        constructors,
        dependency_graph_node2call_graph_node,
    )
}

/// A convenience function to convert a [`DependencyGraph`] into a [`CallGraph`].
/// the caller only needs to provide the data required look-up maps and a node conversion function,
/// while all the low-level machinery is taken care of.
fn dependency_graph2call_graph<F>(
    dependency_graph: &'_ CallableDependencyGraph,
    lifecycles: &HashMap<ResolvedType, Lifecycle>,
    constructors: &IndexMap<ResolvedType, Constructor>,
    dependency_graph_node2call_graph_node: F,
) -> CallGraph
where
    F: Fn(
        &DependencyGraphNode,
        &HashMap<ResolvedType, Lifecycle>,
        &IndexMap<ResolvedType, Constructor>,
    ) -> HandlerCallGraphNode,
{
    let CallableDependencyGraph {
        dependency_graph,
        callable_node_index,
    } = dependency_graph;
    let mut nodes_to_be_visited = vec![VisitorStackElement::orphan(*callable_node_index)];
    let mut call_graph = StableDiGraph::<HandlerCallGraphNode, ()>::new();

    // If the constructor for a type can be invoked at most once, then it should only appear
    // at most once in the call graph. This mapping, and the corresponding Rust closure, are used
    // to make sure of that.
    let mut indexes_for_unique_nodes = HashMap::<u32, NodeIndex>::new();
    let mut add_node_at_most_once =
        |graph: &mut StableDiGraph<HandlerCallGraphNode, ()>,
         call_graph_node: HandlerCallGraphNode,
         dependency_graph_node_index: u32| {
            indexes_for_unique_nodes
                .get(&dependency_graph_node_index)
                .cloned()
                .unwrap_or_else(|| {
                    let index = graph.add_node(call_graph_node);
                    indexes_for_unique_nodes.insert(dependency_graph_node_index, index);
                    index
                })
        };

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
            let call_graph_node =
                dependency_graph_node2call_graph_node(node, lifecycles, constructors);
            match call_graph_node {
                HandlerCallGraphNode::Compute {
                    n_allowed_invocations,
                    ..
                } => match n_allowed_invocations {
                    NumberOfAllowedInvocations::One => {
                        add_node_at_most_once(&mut call_graph, call_graph_node, dep_node_index)
                    }
                    NumberOfAllowedInvocations::Multiple => call_graph.add_node(call_graph_node),
                },
                HandlerCallGraphNode::InputParameter(_) => {
                    add_node_at_most_once(&mut call_graph, call_graph_node, dep_node_index)
                }
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
        root_callable_node_index: indexes_for_unique_nodes[callable_node_index],
    }
}

/// [`CallableDependencyGraph`] is focused on **types** - it tells us what types are needed in
/// order to build the input parameters and invoke a certain callable.
///
/// We now want to convert that knowledge into action.  
/// We want to code-generate a wrapping function for that callable, its **dependency closure**.
/// The dependency closure, leveraging the registered constructors, should either requires no input
/// of its own or ask for "upstream" inputs (i.e. types that are recursive dependencies of the input
/// types for the callable that we want to invoke).
///
/// [`CallableDependencyGraph`] is missing a key information when it comes to code generation:
/// how many times can we invoke the constructors for the types in the dependency graph within
/// the generated dependency closure for our callable?
/// In other words, what is the lifecycle of each of the types built by those constructors?
/// Should there be at most one instance for each invocation? Can we have more than one?
///
/// [`HandlerCallGraph`] captures this information.
///
/// In the dependency graph, each type appears exactly once, no matter how many times it's required
/// as input for other constructors.
/// In the call graph, each constructor appears as many times as it needs to be invoked. A separate
/// node type is used for types that we cannot build, the ones that the callable closure will
/// take as inputs.
///
/// # Example: request handling
///
/// Singletons should be constructed once and re-used throughout the entire lifetime of the
/// application; this implies that the generated code for handling a single request should not
/// call the singleton constructor - it should fetch it from the server state!
/// Request-scoped types, instead, should be built by the request handler closure **at most once**.
/// Transient types can be built multiple times within the lifecycle of each incoming request.
#[derive(Debug)]
pub(crate) struct CallGraph {
    pub(crate) call_graph: StableDiGraph<HandlerCallGraphNode, ()>,
    pub(crate) root_callable_node_index: NodeIndex,
}

#[derive(Clone, Debug)]
pub(crate) enum HandlerCallGraphNode {
    Compute {
        constructor: Constructor,
        n_allowed_invocations: NumberOfAllowedInvocations,
    },
    InputParameter(ResolvedType),
}

#[derive(Clone, Debug, Copy)]
/// How many times can a certain constructor be invoked within the body of
/// the code-generated function?
pub(crate) enum NumberOfAllowedInvocations {
    /// At most once.
    One,
    /// As many times as you want to.
    Multiple,
}

pub(crate) struct VisitorStackElement {
    pub(crate) dependency_graph_index: u32,
    pub(crate) call_graph_parent_index: Option<NodeIndex>,
}

impl VisitorStackElement {
    /// A short-cut to add a node without a parent to the visitor stack.
    pub(crate) fn orphan(dependency_graph_index: u32) -> Self {
        Self {
            dependency_graph_index,
            call_graph_parent_index: None,
        }
    }
}

impl CallGraph {
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
                        HandlerCallGraphNode::Compute { constructor: c, .. } => match c {
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

    /// Return the set of types that must be provided as input to (recursively) build the handler's
    /// input parameters and invoke it.
    ///
    /// We return a `IndexSet` instead of a `HashSet` because we want a consistent ordering for the input
    /// parameters - it will be used in other parts of the crate to provide instances of those types
    /// in the expected order.
    pub fn required_input_types(&self) -> IndexSet<ResolvedType> {
        self.call_graph
            .node_weights()
            .filter_map(|node| match node {
                HandlerCallGraphNode::Compute { .. } => None,
                HandlerCallGraphNode::InputParameter(i) => Some(i),
            })
            .cloned()
            .collect()
    }
}

pub(crate) fn codegen<'a>(
    graph: &CallGraph,
    package_id2name: &BiHashMap<&'a PackageId, String>,
) -> Result<ItemFn, anyhow::Error> {
    let input_parameter_types = graph.required_input_types();
    let CallGraph {
        call_graph,
        root_callable_node_index: handler_node_index,
    } = graph;
    let mut dfs = DfsPostOrder::new(Reversed(call_graph), *handler_node_index);

    let mut parameter_bindings: HashMap<ResolvedType, syn::Ident> = HashMap::new();
    let mut variable_generator = VariableNameGenerator::default();

    let mut scoped_constructors = IndexMap::<NodeIndex, TokenStream>::new();
    let mut blocks = HashMap::<NodeIndex, Fragment>::new();

    while let Some(node_index) = dfs.next(Reversed(call_graph)) {
        let node = &call_graph[node_index];
        match node {
            HandlerCallGraphNode::Compute {
                constructor,
                n_allowed_invocations,
            } => {
                match n_allowed_invocations {
                    NumberOfAllowedInvocations::One => {
                        match constructor {
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
                                            Fragment::BorrowSharedReference(
                                                binding_name.to_owned(),
                                            ),
                                        );
                                    }
                                    Fragment::BorrowSharedReference(_)
                                    | Fragment::Statement(_)
                                    | Fragment::Block(_) => unreachable!(),
                                }
                            }
                        }
                    }
                    NumberOfAllowedInvocations::Multiple => match constructor {
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
        HandlerCallGraphNode::Compute {
            constructor: Constructor::Callable(c),
            ..
        } => c,
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
                HandlerCallGraphNode::Compute { constructor, .. } => constructor.output_type(),
                HandlerCallGraphNode::InputParameter(i) => i,
            };
            (n, type_)
        })
}
