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
use crate::web::error_handlers::ErrorHandler;

/// Build a [`CallGraph`] for the application state.
#[tracing::instrument(name = "compute_application_state_call_graph", skip_all)]
pub(crate) fn application_state_call_graph(
    runtime_singleton_bindings: &BiHashMap<Ident, ResolvedType>,
    lifecycles: &HashMap<ResolvedType, Lifecycle>,
    constructors: IndexMap<ResolvedType, Constructor>,
    constructor2error_handler: &HashMap<Constructor, ErrorHandler>,
) -> CallGraph {
    fn lifecycle2invocations(lifecycle: &Lifecycle) -> Option<NumberOfAllowedInvocations> {
        match lifecycle {
            Lifecycle::Singleton => Some(NumberOfAllowedInvocations::One),
            Lifecycle::RequestScoped => {
                panic!("Singletons should not depend on types with a request-scoped lifecycle.")
            }
            Lifecycle::Transient => {
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
    build_call_graph(
        application_state_constructor,
        lifecycles,
        &constructors,
        constructor2error_handler,
        lifecycle2invocations,
    )
}

/// Build a [`CallGraph`] for a request handler.
#[tracing::instrument(name = "compute_handler_call_graph", skip_all)]
pub(crate) fn handler_call_graph(
    root_callable: Callable,
    lifecycles: &HashMap<ResolvedType, Lifecycle>,
    constructors: &IndexMap<ResolvedType, Constructor>,
    constructor2error_handler: &HashMap<Constructor, ErrorHandler>,
) -> CallGraph {
    fn lifecycle2invocations(l: &Lifecycle) -> Option<NumberOfAllowedInvocations> {
        match l {
            Lifecycle::Singleton => None,
            Lifecycle::RequestScoped => Some(NumberOfAllowedInvocations::One),
            Lifecycle::Transient => Some(NumberOfAllowedInvocations::Multiple),
        }
    }
    build_call_graph(
        root_callable,
        lifecycles,
        constructors,
        constructor2error_handler,
        lifecycle2invocations,
    )
}

/// Build a [`CallGraph`] rooted in `root_callable`.
/// The caller needs to provide the required look-up maps and a function that determines how
/// many times a callable can be invoked given its [`Lifecycle`].
/// All the graph-traversing machinery is taken care of.
fn build_call_graph<F>(
    root_callable: Callable,
    lifecycles: &HashMap<ResolvedType, Lifecycle>,
    constructors: &IndexMap<ResolvedType, Constructor>,
    constructor2error_handler: &HashMap<Constructor, ErrorHandler>,
    lifecycle2n_allowed_invocations: F,
) -> CallGraph
where
    F: Fn(&Lifecycle) -> Option<NumberOfAllowedInvocations> + Clone,
{
    let mut call_graph = StableDiGraph::<CallGraphNode, ()>::new();

    let handler_constructor = Constructor::Callable(root_callable.clone());
    let handler_component: ComputeComponent = handler_constructor.clone().into();
    let handler_node = CallGraphNode::Compute {
        component: handler_component.clone(),
        n_allowed_invocations: NumberOfAllowedInvocations::One,
    };
    let constructor2invocations = |c: &Constructor| {
        lifecycles
            .get(c.output_type())
            .map(lifecycle2n_allowed_invocations.clone())
            .flatten()
    };
    let component2invocations = |component: &ComputeComponent| {
        if component == &handler_component {
            Some(NumberOfAllowedInvocations::One)
        } else {
            match component {
                ComputeComponent::Constructor(c) => constructor2invocations(c),
                ComputeComponent::ErrorHandler(e) => {
                    let fallible_constructor = Constructor::Callable(e.fallible_callable.clone());
                    constructor2invocations(&fallible_constructor)
                }
            }
        }
    };

    let component2node = |c: &ComputeComponent| {
        let n_invocations = component2invocations(c);
        match n_invocations {
            None => CallGraphNode::InputParameter(c.output_type().to_owned()),
            Some(n_allowed_invocations) => CallGraphNode::Compute {
                component: c.to_owned(),
                n_allowed_invocations,
            },
        }
    };

    let mut nodes_to_be_visited = vec![VisitorStackElement::orphan(
        handler_constructor.clone().into(),
    )];

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

    loop {
        while let Some(node_to_be_visited) = nodes_to_be_visited.pop() {
            let (compute_component, neighbour_index) = (
                node_to_be_visited.compute,
                node_to_be_visited.neighbour_index,
            );
            let current_index = {
                let call_graph_node = component2node(&compute_component);
                match call_graph_node {
                    CallGraphNode::Compute {
                        n_allowed_invocations: NumberOfAllowedInvocations::One,
                        ..
                    }
                    | CallGraphNode::InputParameter(_) => {
                        add_node_at_most_once(&mut call_graph, call_graph_node)
                    }
                    CallGraphNode::Compute {
                        n_allowed_invocations: NumberOfAllowedInvocations::Multiple,
                        ..
                    } => call_graph.add_node(call_graph_node),
                    CallGraphNode::MatchBranching => unreachable!(),
                }
            };

            if let Some(neighbour_index) = neighbour_index {
                match neighbour_index {
                    VisitorIndex::Parent(parent_index) => {
                        call_graph.update_edge(parent_index, current_index, ());
                    }
                    VisitorIndex::Child(child_index) => {
                        call_graph.update_edge(current_index, child_index, ());
                    }
                }
            }

            // We need to recursively build the input types for all our compute components;
            if let CallGraphNode::Compute { component, .. } = call_graph[current_index].clone() {
                match component {
                    ComputeComponent::Constructor(constructor) => {
                        let input_types = constructor.input_types();
                        for input_type in input_types.iter() {
                            if let Some(c) = constructors.get(input_type) {
                                nodes_to_be_visited.push(VisitorStackElement {
                                    compute: c.to_owned().into(),
                                    neighbour_index: Some(VisitorIndex::Child(current_index)),
                                });
                            } else {
                                let index = add_node_at_most_once(
                                    &mut call_graph,
                                    CallGraphNode::InputParameter(input_type.to_owned()),
                                );
                                call_graph.update_edge(index, current_index, ());
                            }
                        }
                    }
                    ComputeComponent::ErrorHandler(error_handler) => {
                        let input_types = error_handler.as_ref().inputs.iter();
                        for input_type in input_types {
                            if let Some(c) = constructors.get(input_type) {
                                nodes_to_be_visited.push(VisitorStackElement {
                                    compute: c.to_owned().into(),
                                    neighbour_index: Some(VisitorIndex::Child(current_index)),
                                });
                            } else {
                                if input_type == error_handler.error_type() {
                                    // We have already added this edge.
                                    continue;
                                } else {
                                    let index = add_node_at_most_once(
                                        &mut call_graph,
                                        CallGraphNode::InputParameter(input_type.to_owned()),
                                    );
                                    call_graph.update_edge(index, current_index, ());
                                }
                            }
                        }
                    }
                }
            }
        }

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
        // Constructor(Result) -> MatchResult(Ok)
        // ```
        //
        // to
        //
        // ```
        // Constructor(Result) -> MatchBranching -> MatchResult(Ok)
        //                                       -> MatchResult(Err)
        // ```
        let indexes = call_graph.node_indices().collect::<Vec<_>>();
        for node_index in indexes {
            let node = call_graph[node_index].clone();
            if let CallGraphNode::Compute {
                component: ComputeComponent::Constructor(Constructor::MatchResult(_)),
                n_allowed_invocations,
            } = node
            {
                let parent_node = call_graph
                    .neighbors_directed(node_index, Direction::Incoming)
                    .next()
                    // We know that the `MatchResult` node has exactly one incoming edge because
                    // we have validation in place ensuring that we can't have a constructor for
                    // `T` and a constructor for `Result<T, E>` at the same time (or multiple
                    // `Result<T, _>` constructors using different error types).
                    .unwrap();
                if let CallGraphNode::MatchBranching = call_graph[parent_node] {
                    // This has already been processed.
                    continue;
                }
                let result_node = parent_node;
                let result_constructor = match &call_graph[result_node] {
                    CallGraphNode::Compute {
                        component: ComputeComponent::Constructor(constructor),
                        ..
                    } => constructor.to_owned(),
                    n => {
                        dbg!(n);
                        unreachable!()
                    }
                };
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
                let err_constructor =
                    Constructor::match_result(result_constructor.output_type()).err;
                let err_node_index = call_graph.add_node(CallGraphNode::Compute {
                    component: err_constructor.clone().into(),
                    n_allowed_invocations: n_allowed_invocations.to_owned(),
                });
                call_graph.add_edge(branching_node, err_node_index, ());

                // For each `MatchResult(Err)` node, we want to add a `Compute` node for the respective
                // error handler.
                let error_handler = &constructor2error_handler[&result_constructor];
                let err_ref_node_index = call_graph.add_node(CallGraphNode::Compute {
                    component: ComputeComponent::Constructor(Constructor::shared_borrow(
                        err_constructor.output_type().to_owned(),
                    )),
                    n_allowed_invocations,
                });
                call_graph.add_edge(err_node_index, err_ref_node_index, ());
                nodes_to_be_visited.push(VisitorStackElement {
                    compute: error_handler.to_owned().into(),
                    neighbour_index: Some(VisitorIndex::Parent(err_ref_node_index)),
                });
            }
        }

        if nodes_to_be_visited.is_empty() {
            break;
        }
    }

    // `root_callable_node_index` might point to a `Compute` node that returns a `Result`, therefore
    // it might no longer be without descendants after our insertion of `MatchBranching` nodes.
    // If that's the case, we determine a new `root_callable_node_index` by picking the `Ok`
    // variant that descends from `root_callable_node_index`.
    let root_callable_node_index = indexes_for_unique_nodes[&handler_node];
    let root_callable_node_index = if call_graph
        .neighbors_directed(root_callable_node_index, Direction::Outgoing)
        .count()
        != 0
    {
        let mut dfs = Dfs::new(&call_graph, root_callable_node_index);
        let mut new_root_callable_node_index = root_callable_node_index;
        while let Some(node_index) = dfs.next(&call_graph) {
            if let CallGraphNode::Compute {
                component: ComputeComponent::Constructor(Constructor::MatchResult(m)),
                ..
            } = &call_graph[node_index]
            {
                if m.variant == MatchResultVariant::Ok {
                    new_root_callable_node_index = node_index;
                    break;
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
        component: ComputeComponent,
        n_allowed_invocations: NumberOfAllowedInvocations,
    },
    MatchBranching,
    InputParameter(ResolvedType),
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub(crate) enum ComputeComponent {
    Constructor(Constructor),
    ErrorHandler(ErrorHandler),
}

impl From<Constructor> for ComputeComponent {
    fn from(c: Constructor) -> Self {
        Self::Constructor(c)
    }
}

impl From<ErrorHandler> for ComputeComponent {
    fn from(e: ErrorHandler) -> Self {
        Self::ErrorHandler(e)
    }
}

impl ComputeComponent {
    pub fn output_type(&self) -> &ResolvedType {
        match self {
            ComputeComponent::Constructor(c) => c.output_type(),
            ComputeComponent::ErrorHandler(e) => e.output_type(),
        }
    }
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

#[derive(Debug)]
struct VisitorStackElement {
    compute: ComputeComponent,
    neighbour_index: Option<VisitorIndex>,
}

#[derive(Debug)]
enum VisitorIndex {
    Parent(NodeIndex),
    Child(NodeIndex),
}

impl VisitorStackElement {
    /// A short-cut to add a node without a parent to the visitor stack.
    fn orphan(compute: ComputeComponent) -> Self {
        Self {
            compute,
            neighbour_index: None,
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
                        CallGraphNode::Compute { component: c, .. } => match c {
                            ComputeComponent::Constructor(constructor) => match constructor {
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
                            ComputeComponent::ErrorHandler(e) => {
                                format!(
                                    "label = \"{}\"",
                                    e.as_ref().render_signature(package_ids2names)
                                )
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

/// Return a representation of the [`CallGraph`] in graphviz's .DOT format, geared towards
/// debugging.
#[allow(unused)]
fn debug_dot(g: &StableDiGraph<CallGraphNode, ()>) -> String {
    let config = [
        petgraph::dot::Config::EdgeNoLabel,
        petgraph::dot::Config::NodeNoLabel,
    ];
    format!(
        "{:?}",
        petgraph::dot::Dot::with_attr_getters(
            &g,
            &config,
            &|_, _| "".to_string(),
            &|_, (_, node)| {
                match node {
                    CallGraphNode::Compute { component: c, .. } => match c {
                        ComputeComponent::Constructor(constructor) => match constructor {
                            Constructor::BorrowSharedReference(r) => {
                                format!("label = \"{:?} -> {:?}\"", r.input, r.output)
                            }
                            Constructor::MatchResult(m) => {
                                format!("label = \"{:?} -> {:?}\"", m.input, m.output)
                            }
                            Constructor::Callable(c) => {
                                format!("label = \"{:?}\"", c)
                            }
                        },
                        ComputeComponent::ErrorHandler(e) => {
                            format!("label = \"{:?}\"", e.as_ref())
                        }
                    },
                    CallGraphNode::InputParameter(t) => {
                        format!("label = \"{:?}\"", t)
                    }
                    CallGraphNode::MatchBranching => "label = \"`match`\"".to_string(),
                }
            },
        )
    )
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
            CallGraphNode::Compute {
                component: ComputeComponent::Constructor(c),
                ..
            } => c.output_type(),
            n => {
                dbg!(n);
                unreachable!()
            }
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
                component,
                n_allowed_invocations,
            } => {
                match component {
                    ComputeComponent::Constructor(Constructor::Callable(callable))
                    | ComputeComponent::ErrorHandler(ErrorHandler { callable, .. }) => {
                        let block = codegen_utils::codegen_call_block(
                            get_node_type_inputs(current_index, call_graph),
                            callable,
                            blocks,
                            variable_name_generator,
                            package_id2name,
                        )?;
                        // This is the last node!
                        // We do not need to assign its value to a variable.
                        if current_index == traversal_start_index
                            // Or this is a single-use value, so no point in binding it to a variable.
                            || n_allowed_invocations == &NumberOfAllowedInvocations::Multiple
                        {
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
                            blocks
                                .insert(current_index, Fragment::VariableReference(parameter_name));
                        }
                    }
                    ComputeComponent::Constructor(Constructor::BorrowSharedReference(_)) => {
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
                    ComputeComponent::Constructor(Constructor::MatchResult(_)) => {
                        // We already bound the match result to a variable name when handling
                        // its parent `MatchBranching` node.
                    }
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
                for variant_index in variants {
                    let mut at_most_once_constructor_blocks = IndexMap::new();
                    let mut variant_name_generator = variable_name_generator.clone();
                    let match_binding_parameter_name = variant_name_generator.generate();
                    let mut variant_blocks = {
                        let mut b = blocks.clone();
                        b.insert(
                            variant_index,
                            Fragment::VariableReference(match_binding_parameter_name.clone()),
                        );
                        b
                    };
                    let match_arm_body = _codegen_callable_closure_body(
                        variant_index,
                        call_graph,
                        parameter_bindings,
                        package_id2name,
                        &mut variant_name_generator,
                        &mut at_most_once_constructor_blocks,
                        &mut variant_blocks,
                        dfs,
                    )?;
                    let variant_type = match &call_graph[variant_index] {
                        CallGraphNode::Compute {
                            component: ComputeComponent::Constructor(Constructor::MatchResult(m)),
                            ..
                        } => m.variant,
                        _ => unreachable!(),
                    };
                    let match_arm_binding = match variant_type {
                        MatchResultVariant::Ok => {
                            quote! {
                                Ok(#match_binding_parameter_name)
                            }
                        }
                        MatchResultVariant::Err => {
                            quote! {
                                Err(#match_binding_parameter_name)
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
                let result_binding = &blocks[&result_node_index];
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

fn get_node_type_inputs(
    node_index: NodeIndex,
    call_graph: &StableDiGraph<CallGraphNode, ()>,
) -> impl Iterator<Item = (NodeIndex, &ResolvedType)> {
    call_graph
        .neighbors_directed(node_index, Direction::Incoming)
        .map(|n| {
            let node = &call_graph[n];
            let type_ = match node {
                CallGraphNode::Compute { component, .. } => component.output_type(),
                CallGraphNode::InputParameter(i) => i,
                CallGraphNode::MatchBranching => unreachable!(),
            };
            (n, type_)
        })
}
