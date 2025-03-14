use ahash::{HashMap, HashMapExt};
use indexmap::IndexSet;
use petgraph::Direction;
use petgraph::stable_graph::NodeIndex;

use crate::compiler::analyses::call_graph::borrow_checker::complex::complex_borrow_check;
use crate::compiler::analyses::call_graph::borrow_checker::move_while_borrowed::move_while_borrowed;
use crate::compiler::analyses::call_graph::borrow_checker::multiple_consumers::multiple_consumers;
use crate::compiler::analyses::call_graph::borrow_checker::ownership_relationship::OwnershipRelationships;
use crate::compiler::analyses::call_graph::{CallGraph, OrderedCallGraph};
use crate::compiler::analyses::components::ComponentDb;
use crate::compiler::analyses::computations::ComputationDb;
use crate::rustdoc::CrateCollection;

use super::copy::CopyChecker;

impl OrderedCallGraph {
    /// Build an [`OrderedCallGraph`] from a [`CallGraph`].
    ///
    /// It will first check if the [`CallGraph`] is compatible with the borrow checker, and if it is
    /// not, it will insert new "cloning" nodes into the call graph if it's necessary (i.e. to resolve
    /// a borrow checker error) and possible (i.e. the type of the node implements `Clone`).
    ///
    /// If the violations cannot be remediated, diagnostics will be emitted and an `Err` will be
    /// returned.
    pub fn new(
        call_graph: CallGraph,
        component_db: &mut ComponentDb,
        computation_db: &mut ComputationDb,
        krate_collection: &CrateCollection,
        diagnostics: &mut crate::diagnostic::DiagnosticSink,
    ) -> Result<OrderedCallGraph, ()> {
        let copy_checker = CopyChecker::new(krate_collection);
        let call_graph = Self::borrow_check(
            call_graph,
            &copy_checker,
            component_db,
            computation_db,
            krate_collection,
            diagnostics,
        )?;
        let ordered = Self::order(call_graph, &copy_checker, component_db, computation_db);
        Ok(ordered)
    }

    /// Examine a [`CallGraph`] and check if it is possible to generate code that will
    /// not violate the Rust borrow checker.
    /// It will insert new "cloning" nodes into the call graph if it's necessary (i.e. to resolve
    /// a borrow checker error) and possible (i.e. the type of the node implements `Clone`).
    ///
    /// If the violations cannot be remediated, diagnostics will be emitted and an `Err` will be
    /// returned.
    fn borrow_check(
        call_graph: CallGraph,
        copy_checker: &CopyChecker,
        component_db: &mut ComponentDb,
        computation_db: &mut ComputationDb,
        krate_collection: &CrateCollection,
        diagnostics: &mut crate::diagnostic::DiagnosticSink,
    ) -> Result<CallGraph, ()> {
        let n_diagnostics = diagnostics.len();

        // We first check for "obvious" kind of borrow checking violations
        let call_graph = multiple_consumers(
            call_graph,
            copy_checker,
            component_db,
            computation_db,
            krate_collection,
            diagnostics,
        );
        let call_graph = move_while_borrowed(
            call_graph,
            copy_checker,
            component_db,
            computation_db,
            krate_collection,
            diagnostics,
        );
        // If we find any, we stop here—we risk generating duplicated diagnostics for other
        // (more subtle) violations that will disappear once we fix the "obvious" ones.
        if diagnostics.len() > n_diagnostics {
            return Err(());
        }
        // If there are no "obvious" violations, we check for more subtle ones!
        let call_graph = complex_borrow_check(
            call_graph,
            copy_checker,
            component_db,
            computation_db,
            krate_collection,
            diagnostics,
        );
        if diagnostics.len() > n_diagnostics {
            return Err(());
        }
        Ok(call_graph)
    }

    /// Assign an order to the nodes of a [`CallGraph`].
    ///
    /// It assumes that all borrow checking analyses have already been performed and that, as
    /// a consequence, a suitable ordering *exists*.
    fn order(
        call_graph: CallGraph,
        copy_checker: &CopyChecker,
        component_db: &ComponentDb,
        computation_db: &ComputationDb,
    ) -> Self {
        let CallGraph {
            call_graph,
            root_node_index,
            ..
        } = call_graph;
        let mut node_id2position = HashMap::with_capacity(call_graph.node_count());
        let mut ownership_relationships = OwnershipRelationships::compute(&call_graph);
        let mut position_counter = 0;

        let mut nodes_to_visit: Vec<NodeIndex> =
            Vec::from_iter(call_graph.externals(Direction::Outgoing));
        let mut parked_nodes = IndexSet::new();
        let mut n_finished_nodes = node_id2position.len();
        let mut discovered_nodes = IndexSet::new();
        'fixed_point: loop {
            while let Some(node_index) = nodes_to_visit.pop() {
                if node_id2position.contains_key(&node_index) {
                    // We have already processed this node, we can skip it.
                    continue;
                }
                if discovered_nodes.insert(node_index) {
                    // We just discovered this node: we enqueue its dependencies and re-enqueue
                    // it for processing.
                    nodes_to_visit.push(node_index);
                    nodes_to_visit.extend(
                        call_graph
                            .neighbors_directed(node_index, Direction::Incoming)
                            .filter(|neighbour_index| {
                                !node_id2position.contains_key(neighbour_index)
                            }),
                    );
                    continue;
                }

                // We have already "discovered" this node once, we can now process it.
                let is_blocked = call_graph
                    .neighbors_directed(node_index, Direction::Incoming)
                    .any(|neighbour_index| {
                        // A node is blocked if any of its dependencies:
                        // - has not been processed yet
                        !node_id2position.contains_key(&neighbour_index) || {
                            // - is consumed by the current node and but it must also be borrowed
                            //   by another node that has not been processed yet.
                            let node_relationships = ownership_relationships.node(neighbour_index);
                            let mut is_blocked = node_relationships.is_consumed_by(node_index)
                                && node_relationships.is_borrowed();
                            if is_blocked
                                && copy_checker.is_copy(
                                    &call_graph,
                                    neighbour_index,
                                    component_db,
                                    computation_db,
                                )
                            {
                                is_blocked = false;
                            }
                            is_blocked
                        }
                    });

                if !is_blocked {
                    // If this node borrows from other nodes, remove it from the list of nodes that
                    // borrow from it.
                    ownership_relationships
                        .node(node_index)
                        .remove_all_borrows();

                    node_id2position.insert(node_index, position_counter);
                    position_counter += 1;

                    // Add its dependencies to the list of nodes to be visited next.
                    nodes_to_visit.extend(
                        call_graph
                            .neighbors_directed(node_index, Direction::Incoming)
                            .filter(|neighbour_index| {
                                !(node_id2position.contains_key(neighbour_index)
                                    || parked_nodes.contains(neighbour_index))
                            }),
                    );
                } else {
                    parked_nodes.insert(node_index);
                }
            }

            let current_n_parked_nodes = parked_nodes.len();
            if current_n_parked_nodes == 0 {
                // All good! We are done!
                break 'fixed_point;
            }
            if node_id2position.len() == n_finished_nodes {
                unreachable!(
                    "The fixed point algorithm for node ordering is stuck—this should never happen!"
                )
            } else {
                n_finished_nodes = node_id2position.len();
            }
            // Enqueue the parked nodes into the list of nodes to be visited in the next iteration.
            nodes_to_visit.extend(std::mem::take(&mut parked_nodes));
        }

        Self {
            call_graph,
            root_node_index,
            node2position: node_id2position,
        }
    }
}
