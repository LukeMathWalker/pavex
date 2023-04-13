use ahash::HashMap;
use indexmap::IndexSet;
use petgraph::graph::NodeIndex;

use crate::compiler::analyses::call_graph::{CallGraphEdgeMetadata, RawCallGraph};

#[derive(Debug, Clone, Default)]
/// An helper struct to keep track of the ownership relationships between nodes in a call graph.
///
/// We remove nodes from the various maps as we traverse the graph and process them.
pub(super) struct OwnershipRelationships {
    /// For each node, the set of nodes that it borrows from.
    node_id2borrowed_ids: HashMap<NodeIndex, IndexSet<NodeIndex>>,
    /// For each  node, the set of nodes that borrow from it.
    node_id2borrower_ids: HashMap<NodeIndex, IndexSet<NodeIndex>>,
    /// For each node, the set of nodes that it consumes by value.
    node_id2consumer_ids: HashMap<NodeIndex, IndexSet<NodeIndex>>,
    /// For each node, the set of nodes that consume it by value.
    node_id2consumed_ids: HashMap<NodeIndex, IndexSet<NodeIndex>>,
}

impl OwnershipRelationships {
    /// Bootstrap the relationship map from the underlying call graph.
    pub(super) fn compute(call_graph: &RawCallGraph) -> Self {
        let mut self_ = Self::default();
        for edge_index in call_graph.edge_indices() {
            match call_graph[edge_index] {
                CallGraphEdgeMetadata::SharedBorrow => {
                    let (source, target) = call_graph.edge_endpoints(edge_index).unwrap();
                    self_.node(target).borrows(source);
                }
                CallGraphEdgeMetadata::Move => {
                    let (source, target) = call_graph.edge_endpoints(edge_index).unwrap();
                    self_.node(target).consumes(source);
                }
            }
        }
        self_
    }

    #[must_use]
    /// Zoom in on a single node, either to add new relationship or to query existing ones.
    pub(super) fn node(&mut self, node_index: NodeIndex) -> NodeRelationships {
        NodeRelationships {
            relationships: self,
            node_index,
        }
    }
}

/// See [`OwnershipRelationships::node`] for more details.
pub(super) struct NodeRelationships<'a> {
    relationships: &'a mut OwnershipRelationships,
    node_index: NodeIndex,
}

impl NodeRelationships<'_> {
    /// Add a "borrows" relationship between the current node and the given node index.
    /// It also populates the "borrower" relationship in the other direction.
    pub(super) fn borrows(&mut self, borrowed_node_index: NodeIndex) {
        self.relationships
            .node_id2borrowed_ids
            .entry(self.node_index)
            .or_default()
            .insert(borrowed_node_index);
        self.relationships
            .node_id2borrower_ids
            .entry(borrowed_node_index)
            .or_default()
            .insert(self.node_index);
    }

    /// Add a "consumes" relationship between the current node and the given node index.
    /// It also populates the "consumer" relationship in the other direction.
    pub(super) fn consumes(&mut self, consumed_node_index: NodeIndex) {
        self.relationships
            .node_id2consumed_ids
            .entry(self.node_index)
            .or_default()
            .insert(consumed_node_index);
        self.relationships
            .node_id2consumer_ids
            .entry(consumed_node_index)
            .or_default()
            .insert(self.node_index);
    }

    /// Returns `true` if the current node is borrowed by at least another node.
    pub(super) fn is_borrowed(&self) -> bool {
        self.relationships
            .node_id2borrower_ids
            .get(&self.node_index)
            .map(|borrowers| !borrowers.is_empty())
            .unwrap_or(false)
    }

    /// Returns `true` if the current node is consumed by the provided node index.
    pub(super) fn is_consumed_by(&self, consumer_index: NodeIndex) -> bool {
        self.relationships
            .node_id2consumer_ids
            .get(&self.node_index)
            .map(|consumers| consumers.contains(&consumer_index))
            .unwrap_or(false)
    }

    /// Remove all "borrows" relationships from the current node.
    /// It also removes the "borrower" relationships in the other direction.
    pub(super) fn remove_all_borrows(&mut self) {
        self.relationships
            .node_id2borrowed_ids
            .get_mut(&self.node_index)
            .map(|borrowed| {
                for borrowed_node_index in borrowed.iter().copied() {
                    self.relationships
                        .node_id2borrower_ids
                        .get_mut(&borrowed_node_index)
                        .map(|borrowers| {
                            borrowers.remove(&self.node_index);
                        });
                }
                borrowed.clear();
            });
    }

    /// Remove a consumer relationship with respect to `consumer_index` from the current node, if one exists.
    /// It also removes the "consumed" relationship in the other direction.
    pub(super) fn remove_consumer(&mut self, consumer_index: NodeIndex) {
        self.relationships
            .node_id2consumer_ids
            .get_mut(&self.node_index)
            .map(|consumers| {
                consumers.remove(&consumer_index);
            });
        self.relationships
            .node_id2consumed_ids
            .get_mut(&consumer_index)
            .map(|consumed| {
                consumed.remove(&self.node_index);
            });
    }
}
