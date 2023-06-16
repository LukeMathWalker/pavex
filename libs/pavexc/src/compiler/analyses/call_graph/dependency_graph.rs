use std::fmt::Write;

use ahash::{HashMap, HashMapExt, HashSet, HashSetExt};
use indexmap::IndexSet;
use miette::NamedSource;
use pavex::blueprint::constructor::Lifecycle;
use petgraph::stable_graph::{NodeIndex, StableDiGraph};

use crate::{
    compiler::{
        analyses::{
            components::{ComponentDb, ComponentId, HydratedComponent},
            computations::ComputationDb,
            constructibles::ConstructibleDb,
        },
        computation::Computation,
    },
    diagnostic::CompilerDiagnostic,
    language::ResolvedType,
};

use super::NumberOfAllowedInvocations;

/// A graph that represents the dependencies between components, ignoring their respective lifecycles.
/// There is at most one node for each [`ComponentId`].
///
/// The primary purpose of this graph is to determine if there are cyclic dependencies, which would in
/// turn prevent us from building a [`CallGraph`] without getting stuck in an infinite loop.
pub(super) struct DependencyGraph {
    graph: RawDependencyGraph,
}

impl DependencyGraph {
    /// Build a [`DependencyGraph`] for the `root_id` component.
    #[must_use]
    pub(super) fn build<F>(
        root_id: ComponentId,
        computation_db: &ComputationDb,
        component_db: &ComponentDb,
        constructible_db: &ConstructibleDb,
        lifecycle2n_allowed_invocations: F,
    ) -> Self
    where
        F: Fn(&Lifecycle) -> Option<NumberOfAllowedInvocations> + Clone,
    {
        let mut graph = RawDependencyGraph::new();
        let root_scope_id = component_db.scope_id(root_id);

        let component_id2invocations = |component_id: ComponentId| {
            // We don't expect to invoke this function for response transformers, therefore
            // it's fine to unwrap here.
            let lifecycle = component_db.lifecycle(component_id).unwrap();
            lifecycle2n_allowed_invocations(lifecycle)
        };
        let component_id2node = |id: ComponentId| {
            if let Computation::FrameworkItem(i) = component_db
                .hydrated_component(id, computation_db)
                .computation()
            {
                DependencyGraphNode::Input {
                    type_: i.into_owned(),
                }
            } else {
                match component_id2invocations(id) {
                    None => {
                        let type_ = component_db
                            .hydrated_component(id, computation_db)
                            .output_type()
                            .to_owned();
                        DependencyGraphNode::Input { type_ }
                    }
                    Some(_) => DependencyGraphNode::Compute { component_id: id },
                }
            }
        };

        let mut transformed_node_indexes = HashSet::new();
        let mut handled_error_node_indexes = HashSet::new();
        let mut processed_node_indexes = HashSet::new();
        let mut nodes_to_be_visited: IndexSet<VisitorStackElement> =
            IndexSet::from_iter([VisitorStackElement::orphan(root_id)]);

        // For each component id, we should have at most one node in the dependency graph, no matter the lifecycle.
        let mut node2index = HashMap::<DependencyGraphNode, NodeIndex>::new();
        let mut add_node = |graph: &mut RawDependencyGraph, node: DependencyGraphNode| {
            if node2index.contains_key(&node) {
                node2index[&node]
            } else {
                let index = graph.add_node(node.clone());
                node2index.insert(node, index);
                index
            }
        };

        loop {
            while let Some(node_to_be_visited) = nodes_to_be_visited.pop() {
                let (component_id, neighbour_index) = (
                    node_to_be_visited.component_id,
                    node_to_be_visited.neighbour,
                );
                let current_index = add_node(&mut graph, component_id2node(component_id));
                if let Some(neighbour_index) = neighbour_index {
                    match neighbour_index {
                        VisitorNeighbour::Parent(parent_index) => {
                            graph.update_edge(parent_index, current_index, ());
                        }
                        VisitorNeighbour::Child(child_index) => {
                            graph.update_edge(current_index, child_index, ());
                        }
                    }
                }

                if processed_node_indexes.contains(&current_index) {
                    // We have already processed this node, let's skip its inputs to avoid getting stuck in an infinite loop.
                    continue;
                }

                // We need to recursively build the input types for all our compute components;
                if let DependencyGraphNode::Compute { component_id } = graph[current_index].clone()
                {
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
                        if let Some((constructor_id, _)) = constructible_db.get(
                            component_scope,
                            &input_type,
                            component_db.scope_graph(),
                        ) {
                            nodes_to_be_visited.insert(VisitorStackElement {
                                component_id: constructor_id,
                                neighbour: Some(VisitorNeighbour::Child(current_index)),
                            });
                        } else {
                            let index = add_node(
                                &mut graph,
                                DependencyGraphNode::Input { type_: input_type },
                            );
                            graph.update_edge(index, current_index, ());
                        }
                    }
                }

                processed_node_indexes.insert(current_index);
            }

            // For each node, we try to add a `Compute` node for the respective
            // error handler (if one was registered).
            let indexes = graph.node_indices().collect::<Vec<_>>();
            // We might need to go through multiple cycles of applying transformers
            // until the graph stabilizes (i.e. we reach a fixed point).
            let graph_size_before_transformations = indexes.len();

            for node_index in indexes {
                if handled_error_node_indexes.contains(&node_index) {
                    continue;
                }
                'inner: {
                    let node = graph[node_index].clone();
                    let DependencyGraphNode::Compute {
                    component_id,
                } = node else
                {
                    break 'inner;
                };
                    if let Some(error_handler_id) = component_db.error_handler_id(component_id) {
                        nodes_to_be_visited.insert(VisitorStackElement {
                            component_id: *error_handler_id,
                            neighbour: Some(VisitorNeighbour::Parent(node_index)),
                        });
                    }
                }
                handled_error_node_indexes.insert(node_index);
            }

            // For each node, we add the respective transformers, if they have been registered.
            let indexes = graph.node_indices().collect::<Vec<_>>();
            for node_index in indexes {
                if transformed_node_indexes.contains(&node_index) {
                    continue;
                }
                'inner: {
                    let node = graph[node_index].clone();
                    let DependencyGraphNode::Compute {
                        component_id
                    } = node else {
                        break 'inner;
                    };
                    let Some(transformer_ids) = component_db.transformer_ids(component_id) else {
                        break 'inner;
                    };
                    for transformer_id in transformer_ids {
                        // Not all transformers might be relevant to this `CallGraph`, we need to take their scope into account.
                        let transformer_scope_id = component_db.scope_id(*transformer_id);
                        if root_scope_id
                            .is_descendant_of(transformer_scope_id, component_db.scope_graph())
                        {
                            let transformer_node_index = add_node(
                                &mut graph,
                                DependencyGraphNode::Compute {
                                    component_id: *transformer_id,
                                },
                            );
                            graph.update_edge(node_index, transformer_node_index, ());
                        }
                    }
                }
                transformed_node_indexes.insert(node_index);
            }

            if nodes_to_be_visited.is_empty()
                && graph.node_count() == graph_size_before_transformations
            {
                break;
            }
        }

        Self { graph }
    }

    /// Returns `Ok` if the dependency graph is acyclic, otherwise it emits error diagnostics and returns `Err`.
    pub(super) fn assert_acyclic(
        &self,
        component_db: &ComponentDb,
        computation_db: &ComputationDb,
        diagnostics: &mut Vec<miette::Error>,
    ) -> Result<(), ()> {
        let cycles = find_cycles(&self.graph);

        if !cycles.is_empty() {
            for cycle in cycles {
                diagnostics.push(cycle_error(
                    &self.graph,
                    &cycle,
                    component_db,
                    computation_db,
                ));
            }
            Err(())
        } else {
            Ok(())
        }
    }
}

fn cycle_error(
    graph: &RawDependencyGraph,
    cycle: &[NodeIndex],
    component_db: &ComponentDb,
    computation_db: &ComputationDb,
) -> miette::Error {
    let mut error_msg = "The dependency graph cannot contain cycles, but I just found one!\n\
        If I tried to build your dependencies, I would end up in an infinite loop.\n\n\
        The cycle looks like this:\n"
        .to_string();
    let mut cycle_components = cycle
        .iter()
        .map(|node_index| {
            let node = &graph[*node_index];
            match node {
                DependencyGraphNode::Compute { component_id } => *component_id,
                DependencyGraphNode::Input { .. } => unreachable!(
                    "Input nodes cannot be part of a cycle, since they don't have any incoming edges.\n{node:?}"
                ),
            }
        })
        // We want to skip the "intermediate" result type.
        .filter(|id| !matches!(component_db.hydrated_component(*id, computation_db).computation(), Computation::MatchResult(_)))
        .collect::<Vec<_>>();
    // The dependent will come before the dependency after reversing.
    cycle_components.reverse();

    for (i, dependency_id) in cycle_components.iter().enumerate() {
        writeln!(&mut error_msg, "").unwrap();
        let dependent_id = if i == 0 {
            *cycle_components.last().unwrap()
        } else {
            cycle_components[i - 1]
        };
        let dependency_component = component_db.hydrated_component(*dependency_id, computation_db);
        let mut dependency_type = dependency_component.output_type().to_owned();
        // We want to skip the "intermediate" result type.
        if let Some((ok_id, _)) = component_db.match_ids(*dependency_id) {
            dependency_type = component_db
                .hydrated_component(*ok_id, computation_db)
                .output_type()
                .to_owned();
        }
        let dependency_path = match component_db
            .hydrated_component(*dependency_id, computation_db)
            .computation()
        {
            Computation::Callable(c) => c.path.clone(),
            Computation::MatchResult(_) => unreachable!(),
            Computation::FrameworkItem(_) => unreachable!(
                "Framework items do not have dependencies, so they can't be part of a cycle"
            ),
        };
        let dependent_component = component_db.hydrated_component(dependent_id, computation_db);
        let dependent_path = match dependent_component.computation() {
            Computation::Callable(c) => c.path.clone(),
            Computation::MatchResult(_) => unreachable!(),
            Computation::FrameworkItem(_) => unreachable!(
                "Framework items do not have dependencies, so they can't be part of a cycle"
            ),
        };

        write!(
            &mut error_msg,
            "- `{dependent_path}` depends on `{dependency_type:?}`, which is built by `{dependency_path}`"
        )
        .unwrap();
    }

    let dummy_source = NamedSource::new("".to_string(), "".to_string());
    let diagnostic_builder = CompilerDiagnostic::builder(dummy_source, anyhow::anyhow!(error_msg));

    diagnostic_builder.help(
            "Break the cycle! Remove one of the 'depends-on' relationship by changing the signature of \
             one of the components in the cycle.".into()
        )
        .build()
        .into()
}

/// Return all the cycles in the graph.
///
/// It's an empty vector if the graph is acyclic.
fn find_cycles(graph: &RawDependencyGraph) -> Vec<Vec<NodeIndex>> {
    fn dfs(
        node_index: NodeIndex,
        graph: &RawDependencyGraph,
        visited: &mut HashSet<NodeIndex>,
        stack: &mut Vec<NodeIndex>,
        cycles: &mut Vec<Vec<NodeIndex>>,
    ) {
        visited.insert(node_index);
        stack.push(node_index);

        for neighbour_index in graph.neighbors_directed(node_index, petgraph::Direction::Outgoing) {
            if !visited.contains(&neighbour_index) {
                dfs(neighbour_index, graph, visited, stack, cycles);
            } else if let Some(cycle_start) = stack.iter().position(|&x| x == neighbour_index) {
                let cycle = stack[cycle_start..].to_vec();
                cycles.push(cycle);
            }
        }

        stack.pop();
    }

    let mut visited = HashSet::new();
    let mut stack = Vec::new();
    let mut cycles = Vec::new();

    for node_index in graph.node_indices() {
        if !visited.contains(&node_index) {
            dfs(node_index, graph, &mut visited, &mut stack, &mut cycles);
        }
    }

    cycles
}

pub(super) type RawDependencyGraph = StableDiGraph<DependencyGraphNode, ()>;

pub(super) trait RawDependencyGraphExt {
    fn print_debug_dot(&self, component_db: &ComponentDb, computation_db: &ComputationDb);

    fn debug_dot(&self, component_db: &ComponentDb, computation_db: &ComputationDb) -> String;
}

impl RawDependencyGraphExt for RawDependencyGraph {
    #[allow(unused)]
    fn print_debug_dot(&self, component_db: &ComponentDb, computation_db: &ComputationDb) {
        eprintln!("{}", self.debug_dot(component_db, computation_db));
    }

    #[allow(unused)]
    fn debug_dot(&self, component_db: &ComponentDb, computation_db: &ComputationDb) -> String {
        let config = [
            petgraph::dot::Config::EdgeNoLabel,
            petgraph::dot::Config::NodeNoLabel,
        ];
        format!(
            "{:?}",
            petgraph::dot::Dot::with_attr_getters(
                self,
                &config,
                &|_, edge| "".to_string(),
                &|_, (_, node)| {
                    match node {
                        DependencyGraphNode::Compute { component_id } => {
                            match component_db
                                .hydrated_component(*component_id, computation_db)
                                .computation()
                            {
                                Computation::MatchResult(m) => {
                                    format!("label = \"{:?} -> {:?}\"", m.input, m.output)
                                }
                                Computation::Callable(c) => {
                                    format!("label = \"{c:?}\"")
                                }
                                Computation::FrameworkItem(i) => {
                                    format!("label = \"{i:?}\"")
                                }
                            }
                        }
                        DependencyGraphNode::Input { type_ } => {
                            format!("label = \"{type_:?}\"")
                        }
                    }
                },
            )
        )
    }
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub(super) enum DependencyGraphNode {
    Compute {
        component_id: ComponentId,
    },
    Input {
        /// The type that will be taken as an input parameter by the generated dependency closure.
        type_: ResolvedType,
    },
}

#[derive(Debug, Eq, PartialEq, Hash)]
struct VisitorStackElement {
    component_id: ComponentId,
    neighbour: Option<VisitorNeighbour>,
}

#[derive(Debug, Eq, PartialEq, Hash)]
enum VisitorNeighbour {
    Parent(NodeIndex),
    Child(NodeIndex),
}

impl VisitorStackElement {
    /// A short-cut to add a node without a parent to the visitor stack.
    fn orphan(component_id: ComponentId) -> Self {
        Self {
            component_id,
            neighbour: None,
        }
    }
}
