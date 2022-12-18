use std::collections::HashMap;

use bimap::BiHashMap;
use fixedbitset::FixedBitSet;
use guppy::PackageId;
use indexmap::{IndexMap, IndexSet};
use petgraph::prelude::{DfsPostOrder, StableDiGraph};
use petgraph::stable_graph::NodeIndex;
use petgraph::visit::{Dfs, Reversed};
use petgraph::Direction;
use proc_macro2::{Ident, TokenStream};
use quote::{quote, ToTokens};
use syn::ItemFn;

use pavex_builder::Lifecycle;

use crate::language::{Callable, InvocationStyle, ResolvedPath, ResolvedPathSegment, ResolvedType};
use crate::web::app::GENERATED_APP_PACKAGE_ID;
use crate::web::codegen_utils;
use crate::web::codegen_utils::{Fragment, VariableNameGenerator};
use crate::web::constructors::{Constructor, MatchResultVariant};

/// Build a [`CallGraph`] for the application state.
#[tracing::instrument(name = "compute_application_state_call_graph", skip_all)]
pub(crate) fn application_state_call_graph(
    runtime_singleton_bindings: &BiHashMap<Ident, ResolvedType>,
    lifecycles: &HashMap<ResolvedType, Lifecycle>,
    constructors: IndexMap<ResolvedType, Constructor>,
    constructor2error_handler: &HashMap<Constructor, Callable>,
) -> CallGraph {
    fn constructor2node(
        constructor: &Constructor,
        lifecycles: &HashMap<ResolvedType, Lifecycle>,
    ) -> CallGraphNode {
        match lifecycles.get(constructor.output_type()) {
            Some(Lifecycle::Singleton) => CallGraphNode::Compute {
                constructor: constructor.to_owned(),
                n_allowed_invocations: NumberOfAllowedInvocations::One,
            },
            None => CallGraphNode::InputParameter(constructor.output_type().to_owned()),
            Some(Lifecycle::RequestScoped) => {
                panic!("Singletons should not depend on types with a request-scoped lifecycle.")
            }
            Some(Lifecycle::Transient) => {
                panic!("Singletons should not depend on types with a transient lifecycle.")
            }
        }
    }

    // We build a "mock" callable that has the right inputs in order to drive the machinery
    // that builds the dependency graph.
    let package_id = PackageId::new(GENERATED_APP_PACKAGE_ID);
    let application_state_constructor = Callable {
        is_async: false,
        output: Some(ResolvedType {
            package_id: package_id.clone(),
            rustdoc_id: None,
            base_type: vec!["crate".into(), "ApplicationState".into()],
            generic_arguments: vec![],
            is_shared_reference: false,
        }),
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
    dependency_graph2call_graph(
        application_state_constructor,
        lifecycles,
        &constructors,
        constructor2error_handler,
        constructor2node,
    )
}

/// Build a [`CallGraph`] for a request handler.
#[tracing::instrument(name = "compute_handler_call_graph", skip_all)]
pub(crate) fn handler_call_graph(
    root_callable: Callable,
    lifecycles: &HashMap<ResolvedType, Lifecycle>,
    constructors: &IndexMap<ResolvedType, Constructor>,
    constructor2error_handler: &HashMap<Constructor, Callable>,
) -> CallGraph {
    fn constructor2node(
        constructor: &Constructor,
        lifecycles: &HashMap<ResolvedType, Lifecycle>,
    ) -> CallGraphNode {
        match lifecycles.get(constructor.output_type()) {
            Some(Lifecycle::Singleton) | None => {
                CallGraphNode::InputParameter(constructor.output_type().to_owned())
            }
            Some(Lifecycle::RequestScoped) => CallGraphNode::Compute {
                constructor: constructor.to_owned(),
                n_allowed_invocations: NumberOfAllowedInvocations::One,
            },
            Some(Lifecycle::Transient) => CallGraphNode::Compute {
                constructor: constructor.to_owned(),
                n_allowed_invocations: NumberOfAllowedInvocations::Multiple,
            },
        }
    }
    dependency_graph2call_graph(
        root_callable,
        lifecycles,
        constructors,
        constructor2error_handler,
        constructor2node,
    )
}

/// A convenience function to convert a [`DependencyGraph`] into a [`CallGraph`].
/// The caller needs to provide the data required look-up maps and a node conversion function,
/// while all the graph-traversing machinery is taken care of.
fn dependency_graph2call_graph<F>(
    root_callable: Callable,
    lifecycles: &HashMap<ResolvedType, Lifecycle>,
    constructors: &IndexMap<ResolvedType, Constructor>,
    _constructor2error_handler: &HashMap<Constructor, Callable>,
    constructor2node: F,
) -> CallGraph
where
    F: Fn(&Constructor, &HashMap<ResolvedType, Lifecycle>) -> CallGraphNode,
{
    let mut call_graph = StableDiGraph::<CallGraphNode, ()>::new();

    // A little hack to make sure that the handler node gets assigned to a `Compute` node
    // that is invoked at most once.
    // Definitely not an "elegant" solution, but it works (for now).
    let handler_constructor = Constructor::Callable(root_callable.clone());
    let handler_node = CallGraphNode::Compute {
        constructor: handler_constructor.clone(),
        n_allowed_invocations: NumberOfAllowedInvocations::One,
    };
    let constructor2node = |constructor: &Constructor, lifecycles| {
        if constructor == &handler_constructor {
            handler_node.clone()
        } else {
            constructor2node(constructor, lifecycles)
        }
    };

    let mut nodes_to_be_visited = vec![VisitorStackElement::orphan(handler_constructor.clone())];

    // If the constructor for a type can be invoked at most once, then it should appear
    // at most once in the call graph. This mapping, and the corresponding Rust closure, are used
    // to make sure of that.
    let mut indexes_for_unique_nodes = HashMap::<CallGraphNode, NodeIndex>::new();
    let mut add_node_at_most_once = |graph: &mut StableDiGraph<CallGraphNode, ()>,
                                     node: CallGraphNode| {
        assert!(!matches!(node, CallGraphNode::MatchBranching { .. }));
        indexes_for_unique_nodes
            .get(&node)
            .cloned()
            .unwrap_or_else(|| {
                let index = graph.add_node(node.clone());
                indexes_for_unique_nodes.insert(node, index);
                index
            })
    };

    while let Some(node_to_be_visited) = nodes_to_be_visited.pop() {
        let (constructor, parent_index) = (
            node_to_be_visited.constructor,
            node_to_be_visited.parent_index,
        );
        let current_index = {
            let call_graph_node = constructor2node(&constructor, lifecycles);
            match call_graph_node {
                CallGraphNode::Compute {
                    n_allowed_invocations,
                    ..
                } => match n_allowed_invocations {
                    NumberOfAllowedInvocations::One => {
                        add_node_at_most_once(&mut call_graph, call_graph_node)
                    }
                    NumberOfAllowedInvocations::Multiple => call_graph.add_node(call_graph_node),
                },
                CallGraphNode::InputParameter(_) => {
                    add_node_at_most_once(&mut call_graph, call_graph_node)
                }
                // We do not have `MatchBranching` nodes at this point!
                // They are added later as a second pass.
                CallGraphNode::MatchBranching => unreachable!(),
            }
        };

        if let Some(parent_index) = parent_index {
            call_graph.add_edge(current_index, parent_index, ());
        }

        // We need to recursively build the input types for all our constructors;
        if let CallGraphNode::Compute { constructor, .. } = call_graph[current_index].clone() {
            let input_types = constructor.input_types();
            for input_type in input_types.iter() {
                let input_constructor = if let Some(c) = constructors.get(input_type) {
                    c
                } else {
                    let index = add_node_at_most_once(
                        &mut call_graph,
                        CallGraphNode::InputParameter(input_type.to_owned()),
                    );
                    call_graph.update_edge(index, current_index, ());
                    // If we do not have a constructor for a type, the following things may
                    // happen:
                    // - It will be injected by the framework;
                    // - It will be a required input of the application state constructor;
                    // - pavex will later return an error to the user.
                    continue;
                };
                nodes_to_be_visited.push(VisitorStackElement {
                    constructor: input_constructor.to_owned(),
                    parent_index: Some(current_index),
                });
            }
        }
    }

    let root_callable_node_index = indexes_for_unique_nodes[&handler_node];
    // We traverse the graph looking for `MatchResult` nodes.
    // For each of them:
    // 1. We add a `MatchBranching` node, in between the ancestor `Compute` node for a `Result` type
    //    and the corresponding descendant `MatchResult` node for the `Ok` variant.
    // 2. We add a `MatchResult` node for the `Err` variant, as a descendant of the `MatchBranching`
    //    node.
    //
    // In other words: we want to go from
    //
    // ```
    // Compute(Result) -> MatchResult(Ok)
    // ```
    //
    // to
    //
    // ```
    // Compute(Result) -> MatchBranching -> MatchResult(Ok)
    //                                   -> MatchResult(Err)
    // ```
    let mut err_variant_node2fallible_constructor = HashMap::<NodeIndex, Constructor>::new();
    let mut pre_order_dfs = Dfs::new(Reversed(&call_graph), root_callable_node_index);
    while let Some(node_index) = pre_order_dfs.next(Reversed(&call_graph)) {
        let node = call_graph[node_index].clone();
        if let CallGraphNode::Compute {
            constructor,
            n_allowed_invocations,
        } = node
        {
            if let Constructor::MatchResult(_) = constructor {
                let result_node = call_graph
                    .neighbors_directed(node_index, Direction::Incoming)
                    .next()
                    // We know that the `MatchResult` node has exactly one incoming edge because
                    // we have validation in place ensuring that we can't have a constructor for
                    // `T` and a constructor for `Result<T, E>` at the same time (or multiple
                    // `Result<T, _>` constructors using different error types).
                    .unwrap();
                let branching_node = call_graph.add_node(CallGraphNode::MatchBranching);
                let e = call_graph.find_edge(result_node, node_index).unwrap();
                call_graph.remove_edge(e).unwrap();
                call_graph.add_edge(branching_node, node_index, ());
                call_graph.add_edge(result_node, branching_node, ());

                // At this point we only have the `Ok` node in the graph, not the `Err` node.
                assert_eq!(
                    call_graph
                        .neighbors_directed(result_node, Direction::Outgoing)
                        .count(),
                    1
                );
                let err_constructor = Constructor::match_result(&constructor.input_types()[0]).err;
                let err_node = call_graph.add_node(CallGraphNode::Compute {
                    constructor: err_constructor,
                    n_allowed_invocations: n_allowed_invocations.to_owned(),
                });
                call_graph.add_edge(branching_node, err_node, ());
                err_variant_node2fallible_constructor.insert(err_node, constructor);
            }
        }
    }

    // For each `MatchResult(Err)` node, we want to add a `Compute` node for the respective
    // error handler.
    // let mut nodes_to_be_visited = vec![];
    // let resolved_nodes = call_graph.node_indices().collect::<HashSet<_>>();
    // for (err_match_index, fallible_constructor) in err_variant_node2fallible_constructor {
    //     // Add the error handler node and link it with the `Err(error)` node.
    //     let error_handler = &constructor2error_handler[&fallible_constructor];
    //     let (error_type, n_allowed_invocations) = {
    //         let mut e = match &call_graph[err_match_index] {
    //             CallGraphNode::Compute {
    //                 constructor: Constructor::MatchResult(m),
    //                 n_allowed_invocations,
    //             } => (&m.output, n_allowed_invocations),
    //             _ => unreachable!(),
    //         }
    //         .to_owned();
    //         // The error handler will take a reference to the error type as input!
    //         e.is_shared_reference = true;
    //         e
    //     };
    //     let error_handler_node_index = call_graph.add_node(CallGraphNode::Compute {
    //         constructor: Constructor::Callable(error_handler.to_owned()),
    //         n_allowed_invocations: *n_allowed_invocations,
    //     });
    //     call_graph.update_edge(err_match_index, error_handler_node_index, ());
    //
    //     // Error handlers can leverage dependency injection: we need to make sure we have
    //     // nodes for all their input types.
    //     for input_type in error_handler.inputs.iter() {
    //         let input_type = input_type.to_owned();
    //         if error_type == input_type {
    //             // We already handled this above.
    //             continue;
    //         }
    //         let dependency_graph_node = DependencyGraphNode::Type(input_type);
    //         let call_graph_node = dependency_graph_node2call_graph_node(
    //             &dependency_graph_node,
    //             lifecycles,
    //             constructors,
    //         );
    //         let call_graph_node_index = match call_graph_node {
    //             CallGraphNode::Compute {
    //                 n_allowed_invocations,
    //                 ..
    //             } => match n_allowed_invocations {
    //                 NumberOfAllowedInvocations::One => {
    //                     add_node_at_most_once(&mut call_graph, call_graph_node, dep_node_index)
    //                 }
    //                 NumberOfAllowedInvocations::Multiple => call_graph.add_node(call_graph_node),
    //             },
    //             CallGraphNode::InputParameter(_) => {
    //                 add_node_at_most_once(&mut call_graph, call_graph_node, dep_node_index)
    //             }
    //             // We do not have `MatchBranching` nodes at this point!
    //             // They are added later as a second pass.
    //             CallGraphNode::MatchBranching => unreachable!(),
    //         };
    //         call_graph.update_edge(call_graph_node_index, error_handler_node_index, ());
    //         if !resolved_nodes.contains(&call_graph_node_index) {
    //             nodes_to_be_visited.push((call_graph_node_index, error_handler_node_index));
    //         }
    //     }
    // }
    //
    // for (node_index, parent_node_index) in nodes_to_be_visited {
    //     let node = &call_graph[node_index];
    //     match node {
    //         CallGraphNode::Compute { .. } => {}
    //         CallGraphNode::InputParameter(_) => {
    //             add_node_at_most_once(&mut call_graph, node.to_owned(), dep_node_index)
    //         }
    //         CallGraphNode::MatchBranching => {
    //             unreachable!()
    //         }
    //     }
    //     // Error handlers' inputs can in turn require other input types that
    //     // are not yet part of the graph.
    //     for input_type in error_handler.inputs.iter() {
    //         let input_type = input_type.to_owned();
    //         if error_type == input_type {
    //             // We already handled this above.
    //             continue;
    //         }
    //         let dependency_graph_node = DependencyGraphNode::Type(input_type);
    //         let call_graph_node = dependency_graph_node2call_graph_node(
    //             &dependency_graph_node,
    //             lifecycles,
    //             constructors,
    //         );
    //         let call_graph_node_index = match call_graph_node {
    //             CallGraphNode::Compute {
    //                 n_allowed_invocations,
    //                 ..
    //             } => match n_allowed_invocations {
    //                 NumberOfAllowedInvocations::One => {
    //                     add_node_at_most_once(&mut call_graph, call_graph_node, dep_node_index)
    //                 }
    //                 NumberOfAllowedInvocations::Multiple => call_graph.add_node(call_graph_node),
    //             },
    //             CallGraphNode::InputParameter(_) => {
    //                 add_node_at_most_once(&mut call_graph, call_graph_node, dep_node_index)
    //             }
    //             // We do not have `MatchBranching` nodes at this point!
    //             // They are added later as a second pass.
    //             CallGraphNode::MatchBranching => unreachable!(),
    //         };
    //         call_graph.update_edge(call_graph_node_index, error_handler_node_index, ());
    //         if !resolved_nodes.contains(&call_graph_node_index) {
    //             nodes_to_be_visited.push((call_graph_node_index, error_handler_node_index));
    //         }
    //     }
    // }

    // `callable_node_index` might point to a `Compute` node that returns a `Result`, therefore
    // it might no longer be without descendants after our insertion of `MatchBranching` nodes.
    // If that's the case, we determine a new `root_callable_node_index` by picking the `Ok`
    // variant that descends from `callable_node_index`.
    let root_callable_node_index = indexes_for_unique_nodes[&handler_node];
    let root_callable_node_index = if call_graph
        .neighbors_directed(root_callable_node_index, Direction::Outgoing)
        .count()
        != 0
    {
        let mut dfs = Dfs::new(&call_graph, root_callable_node_index);
        let mut new_root_callable_node_index = root_callable_node_index;
        while let Some(node_index) = dfs.next(&call_graph) {
            if let CallGraphNode::Compute { constructor, .. } = &call_graph[node_index] {
                if let Constructor::MatchResult(m) = constructor {
                    if m.variant == MatchResultVariant::Ok {
                        new_root_callable_node_index = node_index;
                        break;
                    }
                }
            }
        }
        new_root_callable_node_index
    } else {
        root_callable_node_index
    };
    CallGraph {
        call_graph,
        root_callable_node_index,
    }
}

/// [`CallableDependencyGraph`] is focused on **types** - it tells us what types are needed in
/// order to build the input parameters and invoke a certain callable.
///
/// We now want to convert that knowledge into action.  
/// We want to code-generate a wrapping function for that callable, its **dependency closure**.
/// The dependency closure, leveraging the registered constructors, should either require no input
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
    pub(crate) call_graph: StableDiGraph<CallGraphNode, ()>,
    pub(crate) root_callable_node_index: NodeIndex,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub(crate) enum CallGraphNode {
    Compute {
        constructor: Constructor,
        n_allowed_invocations: NumberOfAllowedInvocations,
    },
    MatchBranching,
    InputParameter(ResolvedType),
}

#[derive(Clone, Debug, Copy, Hash, Eq, PartialEq)]
/// How many times can a certain constructor be invoked within the body of
/// the code-generated function?
pub(crate) enum NumberOfAllowedInvocations {
    /// At most once.
    One,
    /// As many times as you want to.
    Multiple,
}

struct VisitorStackElement {
    constructor: Constructor,
    parent_index: Option<NodeIndex>,
}

impl VisitorStackElement {
    /// A short-cut to add a node without a parent to the visitor stack.
    fn orphan(constructor: Constructor) -> Self {
        Self {
            constructor,
            parent_index: None,
        }
    }
}

impl CallGraph {
    /// Return a representation of the [`CallGraph`] in graphviz's .DOT format.
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
                        CallGraphNode::Compute { constructor: c, .. } => match c {
                            Constructor::BorrowSharedReference(r) => {
                                format!(
                                    "label = \"{} -> {}\"",
                                    r.input.render_type(package_ids2names),
                                    r.output.render_type(package_ids2names)
                                )
                            }
                            Constructor::MatchResult(m) => {
                                format!(
                                    "label = \"{} -> {}\"",
                                    m.input.render_type(package_ids2names),
                                    m.output.render_type(package_ids2names)
                                )
                            }
                            Constructor::Callable(c) => {
                                format!("label = \"{}\"", c.render_signature(package_ids2names))
                            }
                        },
                        CallGraphNode::InputParameter(t) => {
                            format!("label = \"{}\"", t.render_type(package_ids2names))
                        }
                        CallGraphNode::MatchBranching => "label = \"`match`\"".to_string(),
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
                CallGraphNode::Compute { .. } | CallGraphNode::MatchBranching => None,
                CallGraphNode::InputParameter(i) => Some(i),
            })
            .cloned()
            .collect()
    }

    /// Generate the code for the dependency closure of the callable at the root of this
    /// [`CallGraph`].
    ///
    /// See [`CallGraph`]'s documentation for more details.
    pub fn codegen<'a>(
        &self,
        package_id2name: &BiHashMap<&'a PackageId, String>,
    ) -> Result<ItemFn, anyhow::Error> {
        codegen_callable_closure(self, package_id2name)
    }
}

/// Generate the dependency closure of the [`CallGraph`]'s root callable.
///
/// See [`CallGraph`] docs for more details.
fn codegen_callable_closure<'a>(
    call_graph: &'a CallGraph,
    package_id2name: &BiHashMap<&'a PackageId, String>,
) -> Result<ItemFn, anyhow::Error> {
    let input_parameter_types = call_graph.required_input_types();
    let mut variable_generator = VariableNameGenerator::new();
    // Assign a unique parameter name to each input parameter type.
    let parameter_bindings: HashMap<ResolvedType, Ident> = input_parameter_types
        .iter()
        .map(|type_| {
            let parameter_name = variable_generator.generate();
            (type_.to_owned(), parameter_name)
        })
        .collect();
    let CallGraph {
        call_graph,
        root_callable_node_index,
    } = call_graph;
    let body = codegen_callable_closure_body(
        *root_callable_node_index,
        call_graph,
        &parameter_bindings,
        package_id2name,
        &mut variable_generator,
    )?;

    let function = {
        let inputs = input_parameter_types.iter().map(|type_| {
            let variable_name = &parameter_bindings[type_];
            let variable_type = type_.syn_type(package_id2name);
            quote! { #variable_name: #variable_type }
        });
        let output_type = match &call_graph[*root_callable_node_index] {
            // TODO: We are working under the happy-path assumption that all terminal nodes
            // in the call graphs (i.e. all nodes with no outgoing edges) are returning the
            // same type. We should verify this assumption before embarking in code generation
            // and return an error if appropriate.
            CallGraphNode::Compute { constructor, .. } => constructor.output_type(),
            CallGraphNode::MatchBranching | CallGraphNode::InputParameter(_) => unreachable!(),
        }
        .syn_type(package_id2name);
        syn::parse2(quote! {
            pub async fn handler(#(#inputs),*) -> #output_type {
                #body
            }
        })
        .unwrap()
    };
    Ok(function)
}

/// Generate the function body for the dependency closure of the [`CallGraph`]'s root callable.
///
/// See [`CallGraph`] docs for more details.
fn codegen_callable_closure_body(
    root_callable_node_index: NodeIndex,
    call_graph: &StableDiGraph<CallGraphNode, ()>,
    parameter_bindings: &HashMap<ResolvedType, Ident>,
    package_id2name: &BiHashMap<&'_ PackageId, String>,
    variable_name_generator: &mut VariableNameGenerator,
) -> Result<TokenStream, anyhow::Error> {
    let mut at_most_once_constructor_blocks = IndexMap::<NodeIndex, TokenStream>::new();
    let mut blocks = HashMap::<NodeIndex, Fragment>::new();
    let mut dfs = DfsPostOrder::new(Reversed(call_graph), root_callable_node_index);
    _codegen_callable_closure_body(
        root_callable_node_index,
        call_graph,
        parameter_bindings,
        package_id2name,
        variable_name_generator,
        &mut at_most_once_constructor_blocks,
        &mut blocks,
        &mut dfs,
    )
}

fn _codegen_callable_closure_body(
    node_index: NodeIndex,
    call_graph: &StableDiGraph<CallGraphNode, ()>,
    parameter_bindings: &HashMap<ResolvedType, Ident>,
    package_id2name: &BiHashMap<&'_ PackageId, String>,
    variable_name_generator: &mut VariableNameGenerator,
    at_most_once_constructor_blocks: &mut IndexMap<NodeIndex, TokenStream>,
    blocks: &mut HashMap<NodeIndex, Fragment>,
    dfs: &mut DfsPostOrder<NodeIndex, FixedBitSet>,
) -> Result<TokenStream, anyhow::Error> {
    let terminal_index = find_terminal_descendant(node_index, call_graph);
    // We want to start the code-generation process from a `MatchBranching` node with
    // no `MatchBranching` predecessors.
    // This ensures that we do not have to look-ahead when generating code for its predecessors.
    let traversal_start_index =
        find_match_branching_ancestor(terminal_index, call_graph, &dfs.finished)
            // If there are no `MatchBranching` nodes in the ancestors sub-graph, we start from the
            // the terminal node.
            .unwrap_or(terminal_index);
    dfs.move_to(traversal_start_index);
    while let Some(current_index) = dfs.next(Reversed(call_graph)) {
        let current_node = &call_graph[current_index];
        match current_node {
            CallGraphNode::Compute {
                constructor,
                n_allowed_invocations,
            } => {
                match n_allowed_invocations {
                    NumberOfAllowedInvocations::One => {
                        match constructor {
                            Constructor::Callable(callable) => {
                                let block = codegen_utils::codegen_call_block(
                                    get_node_type_inputs(current_index, call_graph),
                                    callable,
                                    blocks,
                                    variable_name_generator,
                                    package_id2name,
                                )?;

                                if current_index == traversal_start_index {
                                    // This is the last node!
                                    // We do not need to assign its value to a variable.
                                    blocks.insert(current_index, block);
                                } else {
                                    // We bind the constructed value to a variable name and instruct
                                    // all dependents to refer to the constructed value via that
                                    // variable name.
                                    let parameter_name = variable_name_generator.generate();
                                    let block = quote! {
                                        let #parameter_name = #block;
                                    };
                                    at_most_once_constructor_blocks.insert(current_index, block);
                                    blocks.insert(
                                        current_index,
                                        Fragment::VariableReference(parameter_name),
                                    );
                                }
                            }
                            Constructor::BorrowSharedReference(_) => {
                                let dependencies = call_graph
                                    .neighbors_directed(current_index, Direction::Incoming);
                                let dependency_indexes: Vec<_> = dependencies.collect();
                                assert_eq!(1, dependency_indexes.len());
                                let dependency_index = dependency_indexes.first().unwrap();
                                match &blocks[dependency_index] {
                                    Fragment::VariableReference(binding_name) => {
                                        blocks.insert(
                                            current_index,
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
                            Constructor::MatchResult(_) => {
                                let parameter_name = variable_name_generator.generate();
                                blocks.insert(
                                    current_index,
                                    Fragment::VariableReference(parameter_name),
                                );
                            }
                        }
                    }
                    NumberOfAllowedInvocations::Multiple => match constructor {
                        Constructor::Callable(callable) => {
                            let block = codegen_utils::codegen_call_block(
                                get_node_type_inputs(current_index, call_graph),
                                callable,
                                blocks,
                                variable_name_generator,
                                package_id2name,
                            )?;
                            blocks.insert(current_index, block);
                        }
                        Constructor::BorrowSharedReference(_) => {
                            let dependencies =
                                call_graph.neighbors_directed(current_index, Direction::Incoming);
                            let dependency_indexes: Vec<_> = dependencies.collect();
                            assert_eq!(1, dependency_indexes.len());
                            let dependency_index = dependency_indexes.first().unwrap();
                            match &blocks[dependency_index] {
                                Fragment::VariableReference(binding_name) => {
                                    blocks.insert(
                                        current_index,
                                        Fragment::BorrowSharedReference(binding_name.to_owned()),
                                    );
                                }
                                Fragment::Block(b) => {
                                    blocks.insert(
                                        current_index,
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
                                        current_index,
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
                        Constructor::MatchResult(_) => {
                            let parameter_name = variable_name_generator.generate();
                            blocks
                                .insert(current_index, Fragment::VariableReference(parameter_name));
                        }
                    },
                }
            }
            CallGraphNode::InputParameter(input_type) => {
                let parameter_name = parameter_bindings[input_type].clone();
                blocks.insert(current_index, Fragment::VariableReference(parameter_name));
            }
            CallGraphNode::MatchBranching => {
                let variants = call_graph
                    .neighbors_directed(current_index, Direction::Outgoing)
                    .collect::<Vec<_>>();
                assert_eq!(2, variants.len());
                assert_eq!(current_index, traversal_start_index);
                let mut match_arms = vec![];
                for variant in variants {
                    let mut at_most_once_constructor_blocks = IndexMap::new();
                    let mut blocks = blocks.clone();
                    let match_arm_body = _codegen_callable_closure_body(
                        variant,
                        call_graph,
                        parameter_bindings,
                        package_id2name,
                        variable_name_generator,
                        &mut at_most_once_constructor_blocks,
                        &mut blocks,
                        dfs,
                    )?;
                    let variant_type = match &call_graph[variant] {
                        CallGraphNode::Compute {
                            constructor: Constructor::MatchResult(m),
                            ..
                        } => m.variant,
                        _ => unreachable!(),
                    };
                    let parameter_name = match &blocks[&variant] {
                        Fragment::VariableReference(n) => n,
                        _ => unreachable!(),
                    };
                    let match_arm_binding = match variant_type {
                        MatchResultVariant::Ok => {
                            quote! {
                                Ok(#parameter_name)
                            }
                        }
                        MatchResultVariant::Err => {
                            quote! {
                                Err(#parameter_name)
                            }
                        }
                    };
                    match_arms.push(quote! {
                        #match_arm_binding => {
                            #match_arm_body
                        },
                    });
                }
                let result_node_index = call_graph
                    .neighbors_directed(current_index, Direction::Incoming)
                    .next()
                    .unwrap();
                let result_binding = match &blocks[&result_node_index] {
                    Fragment::VariableReference(name) => name,
                    _ => unreachable!(),
                };
                let block = quote! {
                    {
                        match #result_binding {
                            #(#match_arms)*
                        }
                    }
                };
                blocks.insert(current_index, Fragment::Block(syn::parse2(block).unwrap()));
            }
        }
    }
    let body = {
        let at_most_once_constructors = at_most_once_constructor_blocks.values();
        // Remove the wrapping block, if there is one
        let b = match &blocks[&traversal_start_index] {
            Fragment::Block(b) => {
                let s = &b.stmts;
                quote! { #(#s)* }
            }
            Fragment::Statement(b) => b.to_token_stream(),
            Fragment::VariableReference(n) => n.to_token_stream(),
            _ => {
                unreachable!()
            }
        };
        quote! {
            #(#at_most_once_constructors)*
            #b
        }
    };
    Ok(body)
}

/// Returns a terminal descendant of the given node - i.e. a node that is reachable from
/// `start_index` and has no outgoing edges.
fn find_terminal_descendant(
    start_index: NodeIndex,
    call_graph: &StableDiGraph<CallGraphNode, ()>,
) -> NodeIndex {
    let mut dfs = DfsPostOrder::new(call_graph, start_index);
    while let Some(node_index) = dfs.next(call_graph) {
        let mut successors = call_graph.neighbors_directed(node_index, Direction::Outgoing);
        if successors.next().is_none() {
            return node_index;
        }
    }
    // `call_graph` is a DAG, so we should never reach this point.
    unreachable!()
}

/// Returns `Some(node_index)` if there is an ancestor (either directly or indirectly connected
/// to `start_index`) that is a `CallGraphNode::MatchBranching` and does not belong to `ignore_set`.
/// `node` index won't have any ancestors that are themselves a `CallGraphNode::MatchBranching`.
///
/// Returns `None` if such an ancestor does not exist.
fn find_match_branching_ancestor(
    start_index: NodeIndex,
    call_graph: &StableDiGraph<CallGraphNode, ()>,
    ignore_set: &FixedBitSet,
) -> Option<NodeIndex> {
    let mut ancestors = DfsPostOrder::new(Reversed(call_graph), start_index);
    while let Some(ancestor_index) = ancestors.next(Reversed(call_graph)) {
        if ancestor_index == start_index {
            continue;
        }
        if ignore_set.contains(ancestor_index.index()) {
            continue;
        }
        match &call_graph[ancestor_index] {
            CallGraphNode::MatchBranching { .. } => return Some(ancestor_index),
            _ => {}
        }
    }
    None
}

// pub fn codegen2<'a>(
//     call_graph: &'a CallGraph,
//     package_id2name: &BiHashMap<&'a PackageId, String>,
// ) -> Result<ItemFn, anyhow::Error> {
//     let input_parameter_types = call_graph.required_input_types();
//     let CallGraph {
//         call_graph,
//         root_callable_node_index,
//     } = call_graph;
//     let mut dfs = DfsPostOrder::new(Reversed(call_graph), *root_callable_node_index);
//
//     let mut parameter_bindings: HashMap<ResolvedType, Ident> = HashMap::new();
//     let mut variable_generator = VariableNameGenerator::default();
//
//     let mut scoped_constructors = IndexMap::<NodeIndex, TokenStream>::new();
//     let mut blocks = HashMap::<NodeIndex, Fragment>::new();
//
//     while let Some(node_index) = dfs.next(Reversed(call_graph)) {
//         let node = &call_graph[node_index];
//         match node {
//             CallGraphNode::Compute {
//                 constructor,
//                 n_allowed_invocations,
//             } => {
//                 match n_allowed_invocations {
//                     NumberOfAllowedInvocations::One => {
//                         match constructor {
//                             Constructor::Callable(callable) => {
//                                 let block = codegen_utils::codegen_call_block(
//                                     get_node_type_inputs(node_index, call_graph),
//                                     callable,
//                                     &mut blocks,
//                                     &mut variable_generator,
//                                     package_id2name,
//                                 )?;
//
//                                 let has_dependents = call_graph
//                                     .neighbors_directed(node_index, Direction::Outgoing)
//                                     .next()
//                                     .is_some();
//                                 if !has_dependents {
//                                     // This is a leaf in the call dependency graph for this handler,
//                                     // which implies it is a constructor for the return type!
//                                     // Instead of creating an unnecessary variable binding, we add
//                                     // the raw expression block - which can then be used as-is for
//                                     // returning the value.
//                                     blocks.insert(node_index, block);
//                                 } else {
//                                     // We have at least one dependent, this callable does not return
//                                     // the output type for this handler.
//                                     // We bind the constructed value to a variable name and instruct
//                                     // all dependents to refer to the constructed value via that
//                                     // variable name.
//                                     let parameter_name = variable_generator.generate();
//                                     let block = quote! {
//                                         let #parameter_name = #block;
//                                     };
//                                     scoped_constructors.insert(node_index, block);
//                                     blocks.insert(
//                                         node_index,
//                                         Fragment::VariableReference(parameter_name),
//                                     );
//                                 }
//                             }
//                             Constructor::BorrowSharedReference(_) => {
//                                 let dependencies =
//                                     call_graph.neighbors_directed(node_index, Direction::Incoming);
//                                 let dependency_indexes: Vec<_> = dependencies.collect();
//                                 assert_eq!(1, dependency_indexes.len());
//                                 let dependency_index = dependency_indexes.first().unwrap();
//                                 match &blocks[dependency_index] {
//                                     Fragment::VariableReference(binding_name) => {
//                                         blocks.insert(
//                                             node_index,
//                                             Fragment::BorrowSharedReference(
//                                                 binding_name.to_owned(),
//                                             ),
//                                         );
//                                     }
//                                     Fragment::BorrowSharedReference(_)
//                                     | Fragment::Statement(_)
//                                     | Fragment::Block(_) => unreachable!(),
//                                 }
//                             }
//                         }
//                     }
//                     NumberOfAllowedInvocations::Multiple => match constructor {
//                         Constructor::Callable(callable) => {
//                             let block = codegen_utils::codegen_call_block(
//                                 get_node_type_inputs(node_index, call_graph),
//                                 callable,
//                                 &mut blocks,
//                                 &mut variable_generator,
//                                 package_id2name,
//                             )?;
//                             blocks.insert(node_index, block);
//                         }
//                         Constructor::BorrowSharedReference(_) => {
//                             let dependencies =
//                                 call_graph.neighbors_directed(node_index, Direction::Incoming);
//                             let dependency_indexes: Vec<_> = dependencies.collect();
//                             assert_eq!(1, dependency_indexes.len());
//                             let dependency_index = dependency_indexes.first().unwrap();
//                             match &blocks[dependency_index] {
//                                 Fragment::VariableReference(binding_name) => {
//                                     blocks.insert(
//                                         node_index,
//                                         Fragment::BorrowSharedReference(binding_name.to_owned()),
//                                     );
//                                 }
//                                 Fragment::Block(b) => {
//                                     blocks.insert(
//                                         node_index,
//                                         Fragment::Block(
//                                             syn::parse2(quote! {
//                                                 &#b
//                                             })
//                                             .unwrap(),
//                                         ),
//                                     );
//                                 }
//                                 Fragment::Statement(b) => {
//                                     blocks.insert(
//                                         node_index,
//                                         Fragment::Statement(
//                                             syn::parse2(quote! {
//                                                 &#b;
//                                             })
//                                             .unwrap(),
//                                         ),
//                                     );
//                                 }
//                                 Fragment::BorrowSharedReference(_) => {
//                                     unreachable!()
//                                 }
//                             }
//                         }
//                     },
//                 }
//             }
//             CallGraphNode::InputParameter(input_type) => {
//                 let parameter_name = variable_generator.generate();
//                 parameter_bindings.insert(input_type.to_owned(), parameter_name.clone());
//                 blocks.insert(node_index, Fragment::VariableReference(parameter_name));
//             }
//         }
//     }
//
//     let handler = match &call_graph[*root_callable_node_index] {
//         CallGraphNode::Compute {
//             constructor: Constructor::Callable(c),
//             ..
//         } => c,
//         _ => unreachable!(),
//     };
//     let code = {
//         let inputs = input_parameter_types.iter().map(|type_| {
//             let variable_name = &parameter_bindings[type_];
//             let variable_type = type_.syn_type(package_id2name);
//             quote! { #variable_name: #variable_type }
//         });
//         let output_type = handler
//             .output
//             .as_ref()
//             // TODO: we should make sure that request handlers do not return the unit type.
//             .expect("Request handlers must return something")
//             .syn_type(package_id2name);
//         let scoped_constructors = scoped_constructors.values();
//         let b = match blocks.remove(root_callable_node_index).unwrap() {
//             Fragment::Block(b) => {
//                 let s = b.stmts;
//                 quote! { #(#s)* }
//             }
//             Fragment::Statement(b) => b.to_token_stream(),
//             _ => {
//                 unreachable!()
//             }
//         };
//         syn::parse2(quote! {
//             pub async fn handler(#(#inputs),*) -> #output_type {
//                 #(#scoped_constructors)*
//                 #b
//             }
//         })
//         .unwrap()
//     };
//     Ok(code)
// }

fn get_node_type_inputs(
    node_index: NodeIndex,
    call_graph: &StableDiGraph<CallGraphNode, ()>,
) -> impl Iterator<Item = (NodeIndex, &ResolvedType)> {
    call_graph
        .neighbors_directed(node_index, Direction::Incoming)
        .map(|n| {
            let type_ = match &call_graph[n] {
                CallGraphNode::Compute { constructor, .. } => constructor.output_type(),
                CallGraphNode::InputParameter(i) => i,
                CallGraphNode::MatchBranching => unreachable!(),
            };
            (n, type_)
        })
}
