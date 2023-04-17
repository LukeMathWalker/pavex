use ahash::HashMap;
use bimap::BiHashMap;
use guppy::PackageId;
use indexmap::IndexSet;
use petgraph::stable_graph::NodeIndex;
use syn::ItemFn;

use crate::compiler::analyses::call_graph::codegen::codegen_callable_closure;
use crate::compiler::analyses::call_graph::{
    CallGraphEdgeMetadata, CallGraphNode, RawCallGraph, RawCallGraphExt,
};
use crate::compiler::analyses::components::{ComponentDb, ComponentId};
use crate::compiler::analyses::computations::ComputationDb;
use crate::compiler::computation::Computation;
use crate::language::ResolvedType;

/// A "decorated" [`CallGraph`]—it assigns a position to each node in the graph, introducing
/// a global ordering on the nodes.
/// Walking the graph according to the specified ordering guarantees that the generated code will
/// not violate the Rust borrow checker (bugs aside).
///
/// The following invariant holds: the position of a node is always lower than
/// the position of any of its descendants (since dependencies must be built before the value
/// that depends on them).
///
/// Use [`OrderedCallGraph::new`] to build an [`OrderedCallGraph`] from a [`CallGraph`].
///
/// [`CallGraph`]: crate::compiler::analyses::call_graph::CallGraph
pub(crate) struct OrderedCallGraph {
    pub(crate) call_graph: RawCallGraph,
    pub(crate) root_node_index: NodeIndex,
    pub(crate) node2position: HashMap<NodeIndex, u16>,
}

impl OrderedCallGraph {
    /// Return the [`ComponentId`] of the callable at the root of this [`OrderedCallGraph`].
    pub(crate) fn root_component_id(&self) -> ComponentId {
        match &self.call_graph[self.root_node_index] {
            CallGraphNode::Compute { component_id, .. } => *component_id,
            _ => unreachable!(),
        }
    }

    /// Generate the code for the dependency closure of the callable at the root of this
    /// [`OrderedCallGraph`].
    ///
    /// See [`OrderedCallGraph`]'s documentation for more details.
    pub(crate) fn codegen(
        &self,
        package_id2name: &BiHashMap<PackageId, String>,
        component_db: &ComponentDb,
        computation_db: &ComputationDb,
    ) -> Result<ItemFn, anyhow::Error> {
        codegen_callable_closure(self, package_id2name, component_db, computation_db)
    }

    /// Return the set of types that must be provided as input to (recursively) build the handler's
    /// input parameters and invoke it.
    ///
    /// We return a `IndexSet` instead of a `HashSet` because we want a consistent ordering for the input
    /// parameters—it will be used in other parts of the crate to provide instances of those types
    /// in the expected order.
    pub(crate) fn required_input_types(&self) -> IndexSet<ResolvedType> {
        self.call_graph.required_input_types()
    }

    /// Return a representation of the [`OrderedCallGraph`] in graphviz's .DOT format.
    pub(crate) fn dot(
        &self,
        package_ids2names: &BiHashMap<PackageId, String>,
        component_db: &ComponentDb,
        computation_db: &ComputationDb,
    ) -> String {
        self.call_graph
            .dot(package_ids2names, component_db, computation_db)
    }

    /// Print a representation of the [`OrderedCallGraph`] in graphviz's .DOT format, geared towards
    /// debugging.
    #[allow(unused)]
    pub(crate) fn print_debug_dot(
        &self,
        component_db: &ComponentDb,
        computation_db: &ComputationDb,
    ) {
        eprintln!("{}", self.debug_dot(component_db, computation_db));
    }

    /// Return a representation of the [`OrderedCallGraph`] in graphviz's .DOT format, geared towards
    /// debugging.
    #[allow(unused)]
    pub(crate) fn debug_dot(
        &self,
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
                &|_, edge| match edge.weight() {
                    CallGraphEdgeMetadata::Move => "".to_string(),
                    CallGraphEdgeMetadata::SharedBorrow => "label = \"&\"".to_string(),
                },
                &|_, (id, node)| {
                    let position = self.node2position[&id];
                    match node {
                        CallGraphNode::Compute { component_id, .. } => {
                            match component_db
                                .hydrated_component(*component_id, computation_db)
                                .computation()
                            {
                                Computation::MatchResult(m) => {
                                    format!(
                                        "label = \"{position}| {:?} -> {:?}\"",
                                        m.input, m.output
                                    )
                                }
                                Computation::Callable(c) => {
                                    format!("label = \"{position}| {c:?}\"")
                                }
                                Computation::FrameworkItem(i) => {
                                    format!("label = \"{position}| {i:?}\"")
                                }
                            }
                        }
                        CallGraphNode::InputParameter { type_, .. } => {
                            format!("label = \"{position}| {type_:?}\"")
                        }
                        CallGraphNode::MatchBranching => format!("label = \"{position}| `match`\""),
                    }
                },
            )
        )
    }
}
