use std::borrow::Cow;
use std::collections::HashSet;

use indexmap::IndexMap;

use crate::graphmap::GraphMap;
use crate::language::{Callable, ResolvedType};
use crate::web::constructors::Constructor;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum DependencyGraphNode {
    Compute(Callable),
    Type(ResolvedType),
}

#[derive(Debug, Clone)]
/// This dependency graph is focused on data dependencies - it tells us
/// what types are needed to build the input parameters for a certain callable.
///
/// The dependency graph has two types of nodes:
/// - compute nodes (only the leaf handler, for now);
/// - type nodes.
///
/// We do not store the type constructor for type nodes in the graph itself.  
/// We are working under the assumption that each type has a single type constructor associated with
/// it, therefore there is no ambiguity and type constructors can be looked up when necessary
/// using the `constructors` index map.
///
/// In the dependency graph, each type appears exactly once, no matter how many times it's required
/// as input for other constructors.
pub(crate) struct CallableDependencyGraph {
    pub dependency_graph: GraphMap<DependencyGraphNode>,
    pub callable_node_index: u32,
}

impl CallableDependencyGraph {
    /// Starting from a callable, build up its dependency graph: what types it needs to be fed as
    /// inputs and what types are needed, in turn, to construct those inputs.
    #[tracing::instrument(name = "compute_callable_dependency_graph", skip_all, fields(callable))]
    pub fn new(callable: Callable, constructors: &IndexMap<ResolvedType, Constructor>) -> Self {
        let mut graph = GraphMap::new();
        let callable_node_index = graph.add_node(DependencyGraphNode::Compute(callable));
        let mut stack = vec![callable_node_index];
        let resolved_nodes = HashSet::<u32>::new();
        while let Some(node_index) = stack.pop() {
            let node = &graph[node_index];
            let input_types = match node {
                DependencyGraphNode::Compute(callable) => {
                    Some(Cow::Owned(callable.inputs.to_owned()))
                }
                DependencyGraphNode::Type(type_) => {
                    constructors.get(type_).map(|c| c.input_types())
                }
            };
            if let Some(input_types) = input_types {
                for input_type in input_types.iter() {
                    let input_type = input_type.to_owned();
                    let input_node_index = graph.add_node(DependencyGraphNode::Type(input_type));
                    graph.update_edge(input_node_index, node_index);
                    if !resolved_nodes.contains(&input_node_index) {
                        stack.push(input_node_index);
                    }
                }
            }
        }
        // TODO: check that the graph is acyclical (and rooted in the compute node)
        CallableDependencyGraph {
            dependency_graph: graph,
            callable_node_index,
        }
    }
}
