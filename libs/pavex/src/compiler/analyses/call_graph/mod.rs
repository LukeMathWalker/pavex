use std::borrow::Cow;

use ahash::{HashMap, HashMapExt, HashSet, HashSetExt};
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

pub(crate) use application_state::{application_state_call_graph, ApplicationStateCallGraph};
use pavex_builder::constructor::Lifecycle;
pub(crate) use request_handler::handler_call_graph;

use crate::compiler::analyses::components::{ComponentDb, ComponentId, HydratedComponent};
use crate::compiler::analyses::computations::ComputationDb;
use crate::compiler::analyses::constructibles::ConstructibleDb;
use crate::compiler::codegen_utils;
use crate::compiler::codegen_utils::{Fragment, VariableNameGenerator};
use crate::compiler::computation::{BorrowSharedReference, Computation, MatchResultVariant};
use crate::compiler::constructors::Constructor;
use crate::language::ResolvedType;

mod application_state;
mod borrow_checker;
mod request_handler;

/// Build a [`CallGraph`] rooted in the `root_id` component.
/// The caller needs to provide the required look-up maps and a function that determines how
/// many times a callable can be invoked given its [`Lifecycle`].
/// All the graph-traversing machinery is taken care of.
fn build_call_graph<F>(
    root_id: ComponentId,
    computation_db: &ComputationDb,
    component_db: &ComponentDb,
    constructible_db: &ConstructibleDb,
    lifecycle2n_allowed_invocations: F,
) -> CallGraph
where
    F: Fn(&Lifecycle) -> Option<NumberOfAllowedInvocations> + Clone,
{
    let mut call_graph = StableDiGraph::<CallGraphNode, ()>::new();

    let component_id2invocations = |component_id: ComponentId| {
        // We don't expect to invoke this function for response transformers, therefore
        // it's fine to unwrap here.
        let lifecycle = component_db.lifecycle(component_id).unwrap();
        lifecycle2n_allowed_invocations(lifecycle)
    };
    let component_id2node = |id: ComponentId| {
        let n_invocations = component_id2invocations(id);
        match n_invocations {
            None => {
                let resolved_component = component_db.hydrated_component(id, computation_db);
                CallGraphNode::InputParameter(resolved_component.output_type().to_owned())
            }
            Some(n_allowed_invocations) => CallGraphNode::Compute {
                component_id: id,
                n_allowed_invocations,
            },
        }
    };

    let mut transformed_node_indexes = HashSet::new();
    let mut handled_error_node_indexes = HashSet::new();

    let mut nodes_to_be_visited: IndexSet<VisitorStackElement> =
        IndexSet::from_iter([VisitorStackElement::orphan(root_id)]);

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
            let (component_id, neighbour_index) = (
                node_to_be_visited.component_id,
                node_to_be_visited.neighbour_index,
            );
            let current_index = {
                let call_graph_node = component_id2node(component_id);
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
            if let CallGraphNode::Compute { component_id, .. } = call_graph[current_index].clone() {
                let component = component_db.hydrated_component(component_id, computation_db);
                let component_scope = component_db.scope_id(component_id);
                let input_types = match component {
                    HydratedComponent::Constructor(constructor) => {
                        constructor.input_types().to_vec()
                    }
                    HydratedComponent::RequestHandler(r) => r.input_types().to_vec(),
                    HydratedComponent::ErrorHandler(error_handler) => error_handler
                        .input_types()
                        .iter()
                        // We have already added the error -> error handler edge at this stage.
                        .filter(|&t| error_handler.error_type_ref() != t)
                        .map(|t| t.to_owned())
                        .collect(),
                    HydratedComponent::Transformer(_) => {
                        // We don't allow/need dependency injection for transformers at the moment.
                        vec![]
                    }
                };
                for input_type in input_types {
                    if let Some(constructor_id) = constructible_db.get(
                        component_scope,
                        &input_type,
                        component_db.scope_graph(),
                    ) {
                        nodes_to_be_visited.insert(VisitorStackElement {
                            component_id: constructor_id,
                            neighbour_index: Some(VisitorIndex::Child(current_index)),
                        });
                    } else {
                        let index = add_node_at_most_once(
                            &mut call_graph,
                            CallGraphNode::InputParameter(input_type),
                        );
                        call_graph.update_edge(index, current_index, ());
                    }
                }
            }
        }

        // For each node, we try to add a `Compute` node for the respective
        // error handler (if one was registered).
        let indexes = call_graph.node_indices().collect::<Vec<_>>();
        // We might need to go through multiple cycles of applying transformers
        // until the graph stabilizes (i.e. we reach a fixed point).
        let graph_size_before_transformations = indexes.len();

        for node_index in indexes {
            if handled_error_node_indexes.contains(&node_index) {
                continue;
            }
            'inner: {
                let node = call_graph[node_index].clone();
                let CallGraphNode::Compute {
                    component_id,
                    ..
                } = node else
                {
                    break 'inner;
                };
                if let Some(error_handler_id) = component_db.error_handler_id(component_id) {
                    nodes_to_be_visited.insert(VisitorStackElement {
                        component_id: *error_handler_id,
                        neighbour_index: Some(VisitorIndex::Parent(node_index)),
                    });
                }
            }
            handled_error_node_indexes.insert(node_index);
        }

        // For each node, we add the respective transformers, if they have been registered.
        let indexes = call_graph.node_indices().collect::<Vec<_>>();
        for node_index in indexes {
            if transformed_node_indexes.contains(&node_index) {
                continue;
            }
            'inner: {
                let node = call_graph[node_index].clone();
                let CallGraphNode::Compute {
                    component_id, n_allowed_invocations,
                } = node else {
                    break 'inner;
                };
                let Some(transformer_ids) = component_db.transformer_ids(component_id) else {
                    break 'inner;
                };
                for transformer_id in transformer_ids {
                    let transformer_node_index = call_graph.add_node(CallGraphNode::Compute {
                        component_id: *transformer_id,
                        n_allowed_invocations,
                    });
                    call_graph.update_edge(node_index, transformer_node_index, ());
                }
            }
            transformed_node_indexes.insert(node_index);
        }

        if nodes_to_be_visited.is_empty()
            && call_graph.node_count() == graph_size_before_transformations
        {
            break;
        }
    }

    // We traverse the graph looking for fallible compute nodes.
    // For each of them we add a `MatchBranching` node, in between the ancestor `Compute` node
    // for a `Result` type and the corresponding descendants `MatchResult` nodes.
    //
    // In other words: we want to go from
    //
    // ```
    // Compute(Result) -> MatchResult(Ok)
    //                    MatchResult(Err)
    // ```
    //
    // to
    //
    // ```
    // Compute(Result) -> MatchBranching -> MatchResult(Ok)
    //                                   -> MatchResult(Err)
    // ```
    let indexes = call_graph.node_indices().collect::<Vec<_>>();
    for node_index in indexes {
        let node = call_graph[node_index].clone();
        let CallGraphNode::Compute {
            component_id, ..
        } = node else {
            continue;
        };
        let Some((ok_match_id, err_match_id)) = component_db.match_ids(component_id) else {
            // We only want to look at fallible components
            continue;
        };
        let child_node_indexes = call_graph
            .neighbors_directed(node_index, Direction::Outgoing)
            .collect::<Vec<_>>();
        assert_eq!(
            child_node_indexes.len(),
            2,
            "Fallible nodes should have either none, one or two children. This is not the case for node {:?}.\nGraph:\n{}",
            node_index,
            debug_dot(&call_graph, component_db, computation_db)
        );

        let first_child_node_index = child_node_indexes[0];
        let second_child_node_index = child_node_indexes[1];
        for idx in [first_child_node_index, second_child_node_index] {
            match &call_graph[idx] {
                CallGraphNode::Compute { component_id, .. } => {
                    assert!([ok_match_id, err_match_id].contains(&component_id));
                }
                _ => unreachable!(),
            }
        }

        let branching_node_index = call_graph.add_node(CallGraphNode::MatchBranching);
        let first_edge = call_graph
            .find_edge(node_index, first_child_node_index)
            .unwrap();
        let second_edge = call_graph
            .find_edge(node_index, second_child_node_index)
            .unwrap();
        call_graph.remove_edge(first_edge).unwrap();
        call_graph.remove_edge(second_edge).unwrap();
        call_graph.add_edge(branching_node_index, first_child_node_index, ());
        call_graph.add_edge(branching_node_index, second_child_node_index, ());
        call_graph.add_edge(node_index, branching_node_index, ());
    }

    // `root_node_index` might point to a `Compute` node that returns a `Result`, therefore
    // it might no longer be without descendants after our insertion of `MatchBranching` nodes.
    // If that's the case, we determine a new `root_node_index` by picking the `Ok`
    // variant that descends from `root_node_index`.
    let root_node_index = {
        // Very hacky way of getting the index of the root node
        let root_node = CallGraphNode::Compute {
            component_id: root_id,
            n_allowed_invocations: NumberOfAllowedInvocations::One,
        };
        add_node_at_most_once(&mut call_graph, root_node)
    };
    let root_node_index = if call_graph
        .neighbors_directed(root_node_index, Direction::Outgoing)
        .count()
        != 0
    {
        let mut dfs = Dfs::new(&call_graph, root_node_index);
        let mut new_root_index = root_node_index;
        while let Some(node_index) = dfs.next(&call_graph) {
            let node = &call_graph[node_index];
            if let CallGraphNode::Compute { component_id, .. } = node {
                if let HydratedComponent::Transformer(Computation::MatchResult(m)) =
                    component_db.hydrated_component(*component_id, computation_db)
                {
                    if m.variant == MatchResultVariant::Err {
                        continue;
                    }
                }
                new_root_index = node_index;
            }
        }
        new_root_index
    } else {
        root_node_index
    };
    CallGraph {
        call_graph,
        root_node_index,
    }
}

/// [`CallableDependencyGraph`] is focused on **types**—it tells us what types are needed in
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
/// node type is used for types that we can't build, the ones that the callable closure will
/// take as inputs.
///
/// # Example: request handling
///
/// Singletons should be constructed once and re-used throughout the entire lifetime of the
/// application; this implies that the generated code for handling a single request should not
/// call the singleton constructor—it should fetch it from the server state!
/// Request-scoped types, instead, should be built by the request handler closure **at most once**.
/// Transient types can be built multiple times within the lifecycle of each incoming request.
#[derive(Debug)]
pub(crate) struct CallGraph {
    pub(crate) call_graph: StableDiGraph<CallGraphNode, ()>,
    pub(crate) root_node_index: NodeIndex,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub(crate) enum CallGraphNode {
    Compute {
        component_id: ComponentId,
        n_allowed_invocations: NumberOfAllowedInvocations,
    },
    MatchBranching,
    InputParameter(ResolvedType),
}

impl CallGraphNode {
    pub fn as_hydrated_component<'a>(
        &self,
        component_db: &'a ComponentDb,
        computation_db: &'a ComputationDb,
    ) -> Option<HydratedComponent<'a>> {
        let CallGraphNode::Compute { component_id, .. } = self else {
            return None;
        };
        Some(component_db.hydrated_component(*component_id, computation_db))
    }

    pub fn as_borrow_computation<'a>(
        &self,
        component_db: &'a ComponentDb,
        computation_db: &'a ComputationDb,
    ) -> Option<Cow<'a, BorrowSharedReference>> {
        let component = self.as_hydrated_component(component_db, computation_db)?;
        match component {
            HydratedComponent::Transformer(Computation::BorrowSharedReference(b))
            | HydratedComponent::Constructor(Constructor(Computation::BorrowSharedReference(b))) => {
                Some(b)
            }
            _ => None,
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

#[derive(Debug, Eq, PartialEq, Hash)]
struct VisitorStackElement {
    component_id: ComponentId,
    neighbour_index: Option<VisitorIndex>,
}

#[derive(Debug, Eq, PartialEq, Hash)]
enum VisitorIndex {
    Parent(NodeIndex),
    Child(NodeIndex),
}

impl VisitorStackElement {
    /// A short-cut to add a node without a parent to the visitor stack.
    fn orphan(component_id: ComponentId) -> Self {
        Self {
            component_id,
            neighbour_index: None,
        }
    }
}

impl CallGraph {
    /// Return a representation of the [`CallGraph`] in graphviz's .DOT format.
    pub fn dot(
        &self,
        package_ids2names: &BiHashMap<PackageId, String>,
        component_db: &ComponentDb,
        computation_db: &ComputationDb,
    ) -> String {
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
                        CallGraphNode::Compute { component_id, .. } => match component_db
                            .hydrated_component(*component_id, computation_db)
                        {
                            HydratedComponent::Constructor(Constructor(t))
                            | HydratedComponent::Transformer(t) => match t {
                                Computation::Callable(c) => {
                                    format!("label = \"{}\"", c.render_signature(package_ids2names))
                                }
                                Computation::BorrowSharedReference(r) => {
                                    format!(
                                        "label = \"{} -> {}\"",
                                        r.input.render_type(package_ids2names),
                                        r.output.render_type(package_ids2names)
                                    )
                                }
                                Computation::MatchResult(m) => {
                                    format!(
                                        "label = \"{} -> {}\"",
                                        m.input.render_type(package_ids2names),
                                        m.output.render_type(package_ids2names)
                                    )
                                }
                            },
                            HydratedComponent::ErrorHandler(e) => {
                                format!(
                                    "label = \"{}\"",
                                    e.callable.render_signature(package_ids2names)
                                )
                            }
                            HydratedComponent::RequestHandler(r) => {
                                format!(
                                    "label = \"{}\"",
                                    r.callable.render_signature(package_ids2names)
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
    /// parameters—it will be used in other parts of the crate to provide instances of those types
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

    /// Return the [`ComponentId`] of the callable at the root of this [`CallGraph`].
    pub fn root_component_id(&self) -> ComponentId {
        match &self.call_graph[self.root_node_index] {
            CallGraphNode::Compute { component_id, .. } => *component_id,
            _ => unreachable!(),
        }
    }

    /// Generate the code for the dependency closure of the callable at the root of this
    /// [`CallGraph`].
    ///
    /// See [`CallGraph`]'s documentation for more details.
    pub fn codegen(
        &self,
        package_id2name: &BiHashMap<PackageId, String>,
        component_db: &ComponentDb,
        computation_db: &ComputationDb,
    ) -> Result<ItemFn, anyhow::Error> {
        codegen_callable_closure(self, package_id2name, component_db, computation_db)
    }
}

/// Return a representation of the [`CallGraph`] in graphviz's .DOT format, geared towards
/// debugging.
#[allow(unused)]
pub(crate) fn debug_dot(
    g: &StableDiGraph<CallGraphNode, ()>,
    component_db: &ComponentDb,
    computation_db: &ComputationDb,
) -> String {
    let config = [
        petgraph::dot::Config::EdgeNoLabel,
        petgraph::dot::Config::NodeNoLabel,
    ];
    format!(
        "{:?}",
        petgraph::dot::Dot::with_attr_getters(
            g,
            &config,
            &|_, _| "".to_string(),
            &|_, (_, node)| {
                match node {
                    CallGraphNode::Compute { component_id, .. } => {
                        match component_db.hydrated_component(*component_id, computation_db) {
                            HydratedComponent::ErrorHandler(e) => {
                                format!("label = \"{:?}\"", e.callable)
                            }
                            HydratedComponent::Transformer(t)
                            | HydratedComponent::Constructor(Constructor(t)) => match t {
                                Computation::MatchResult(m) => {
                                    format!("label = \"{:?} -> {:?}\"", m.input, m.output)
                                }
                                Computation::BorrowSharedReference(b) => {
                                    format!("label = \"{:?} -> {:?}\"", b.input, b.output)
                                }
                                Computation::Callable(c) => {
                                    format!("label = \"{c:?}\"")
                                }
                            },
                            HydratedComponent::RequestHandler(r) => {
                                format!("label = \"{:?}\"", r.callable)
                            }
                        }
                    }
                    CallGraphNode::InputParameter(t) => {
                        format!("label = \"{t:?}\"")
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
fn codegen_callable_closure(
    call_graph: &CallGraph,
    package_id2name: &BiHashMap<PackageId, String>,
    component_db: &ComponentDb,
    computation_db: &ComputationDb,
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
        root_node_index: root_callable_node_index,
    } = call_graph;
    let body = codegen_callable_closure_body(
        *root_callable_node_index,
        call_graph,
        &parameter_bindings,
        package_id2name,
        component_db,
        computation_db,
        &mut variable_generator,
    )?;

    let function = {
        let inputs = input_parameter_types.iter().map(|type_| {
            let variable_name = &parameter_bindings[type_];
            let variable_type = type_.syn_type(package_id2name);
            quote! { #variable_name: #variable_type }
        });
        let component_id = match &call_graph[*root_callable_node_index] {
            CallGraphNode::Compute { component_id, .. } => component_id,
            n => {
                dbg!(n);
                unreachable!()
            }
        };
        let output_type = component_db
            .hydrated_component(*component_id, computation_db)
            .output_type()
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
    package_id2name: &BiHashMap<PackageId, String>,
    component_db: &ComponentDb,
    computation_db: &ComputationDb,
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
        component_db,
        computation_db,
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
    package_id2name: &BiHashMap<PackageId, String>,
    component_db: &ComponentDb,
    computation_db: &ComputationDb,
    variable_name_generator: &mut VariableNameGenerator,
    at_most_once_constructor_blocks: &mut IndexMap<NodeIndex, TokenStream>,
    blocks: &mut HashMap<NodeIndex, Fragment>,
    dfs: &mut DfsPostOrder<NodeIndex, FixedBitSet>,
) -> Result<TokenStream, anyhow::Error> {
    let terminal_index = find_terminal_descendant(node_index, call_graph);
    // We want to start the code-generation process from a `MatchBranching` node with
    // no `MatchBranching` predecessors.
    // This ensures that we don't have to look-ahead when generating code for its predecessors.
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
                component_id,
                n_allowed_invocations,
            } => {
                let component = component_db.hydrated_component(*component_id, computation_db);
                let computation = match component {
                    HydratedComponent::Constructor(c) => c.0,
                    HydratedComponent::RequestHandler(h) => h.callable.into(),
                    HydratedComponent::ErrorHandler(e) => e.callable.to_owned().into(),
                    HydratedComponent::Transformer(t) => t,
                };
                match computation {
                    Computation::Callable(callable) => {
                        let block = codegen_utils::codegen_call_block(
                            get_node_type_inputs(
                                current_index,
                                call_graph,
                                component_db,
                                computation_db,
                            ),
                            callable.as_ref(),
                            blocks,
                            variable_name_generator,
                            package_id2name,
                        )?;
                        // This is the last node!
                        // We don't need to assign its value to a variable.
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
                    Computation::BorrowSharedReference(_) => {
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
                    Computation::MatchResult(_) => {
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
                let mut ok_arm = None;
                let mut err_arm = None;
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
                    // This `.clone()` is **load-bearing**.
                    // The sub-graph for each match arm might have one or more nodes in common.
                    // If we don't create a new DFS for each match arm, the visitor will only
                    // pick up the shared nodes once (for the first match arm), leading to issues
                    // when generating code for the second match arm (i.e. most likely a panic).
                    let mut new_dfs = dfs.clone();
                    let match_arm_body = _codegen_callable_closure_body(
                        variant_index,
                        call_graph,
                        parameter_bindings,
                        package_id2name,
                        component_db,
                        computation_db,
                        &mut variant_name_generator,
                        &mut at_most_once_constructor_blocks,
                        &mut variant_blocks,
                        &mut new_dfs,
                    )?;
                    let variant_type = match &call_graph[variant_index] {
                        CallGraphNode::Compute { component_id, .. } => {
                            match component_db.hydrated_component(*component_id, computation_db) {
                                HydratedComponent::Transformer(Computation::MatchResult(m))
                                | HydratedComponent::Constructor(Constructor(
                                    Computation::MatchResult(m),
                                )) => m.variant,
                                _ => unreachable!(),
                            }
                        }
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
                    let match_arm = quote! {
                        #match_arm_binding => {
                            #match_arm_body
                        },
                    };
                    match variant_type {
                        MatchResultVariant::Ok => {
                            ok_arm = Some(match_arm);
                        }
                        MatchResultVariant::Err => err_arm = Some(match_arm),
                    }
                }
                // We do this to make sure that the Ok arm is always before the Err arm in the
                // generated code.
                let ok_arm = ok_arm.unwrap();
                let err_arm = err_arm.unwrap();
                let result_node_index = call_graph
                    .neighbors_directed(current_index, Direction::Incoming)
                    .next()
                    .unwrap();
                let result_binding = &blocks[&result_node_index];
                let block = quote! {
                    {
                        match #result_binding {
                            #ok_arm
                            #err_arm
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

/// Returns a terminal descendant of the given node—i.e. a node that is reachable from
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
/// to `start_index`) that is a `CallGraphNode::MatchBranching` and doesn't belong to `ignore_set`.
/// `node` index won't have any ancestors that are themselves a `CallGraphNode::MatchBranching`.
///
/// Returns `None` if such an ancestor doesn't exist.
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
        if let CallGraphNode::MatchBranching { .. } = &call_graph[ancestor_index] {
            return Some(ancestor_index);
        }
    }
    None
}

fn get_node_type_inputs<'a, 'b: 'a>(
    node_index: NodeIndex,
    call_graph: &'a StableDiGraph<CallGraphNode, ()>,
    component_db: &'b ComponentDb,
    computation_db: &'b ComputationDb,
) -> impl Iterator<Item = (NodeIndex, ResolvedType)> + 'a {
    call_graph
        .neighbors_directed(node_index, Direction::Incoming)
        .map(|n| {
            let node = &call_graph[n];
            let type_ = match node {
                CallGraphNode::Compute { component_id, .. } => {
                    let component = component_db.hydrated_component(*component_id, computation_db);
                    component.output_type().to_owned()
                }
                CallGraphNode::InputParameter(i) => i.to_owned(),
                CallGraphNode::MatchBranching => unreachable!(),
            };
            (n, type_)
        })
}
