use guppy::graph::PackageGraph;
use petgraph::stable_graph::NodeIndex;

use crate::{
    compiler::{
        analyses::{
            call_graph::{CallGraphNode, RawCallGraph},
            components::ComponentDb,
            computations::ComputationDb,
        },
        traits::assert_trait_is_implemented,
        utils::process_framework_path,
    },
    language::{PathType, ResolvedType},
    rustdoc::CrateCollection,
};

/// Determine if a node in the [`CallGraph`] implements the [`Copy`] trait.
pub(super) struct CopyChecker<'a> {
    copy_trait: PathType,
    krate_collection: &'a CrateCollection,
}

impl<'a> CopyChecker<'a> {
    pub(super) fn new(
        package_graph: &'a PackageGraph,
        krate_collection: &'a CrateCollection,
    ) -> Self {
        let copy_trait = get_copy_trait(package_graph, krate_collection);
        Self {
            copy_trait,
            krate_collection,
        }
    }

    /// Returns `true` if a type implements the `Copy` trait. `false` otherwise (or if we can't determine it).
    pub(super) fn is_copy(
        &'a self,
        call_graph: &RawCallGraph,
        node_index: NodeIndex,
        component_db: &ComponentDb,
        computation_db: &ComputationDb,
    ) -> bool {
        match &call_graph[node_index] {
            CallGraphNode::Compute { component_id, .. } => {
                let component = component_db.hydrated_component(*component_id, computation_db);
                assert_trait_is_implemented(
                    self.krate_collection,
                    component.output_type(),
                    &self.copy_trait,
                )
                .is_ok()
            }
            CallGraphNode::MatchBranching => true,
            CallGraphNode::InputParameter { type_, .. } => {
                assert_trait_is_implemented(self.krate_collection, type_, &self.copy_trait).is_ok()
            }
        }
    }
}

/// Return the `PathType` object for the `Copy` marker trait.
fn get_copy_trait(package_graph: &PackageGraph, krate_collection: &CrateCollection) -> PathType {
    let c = process_framework_path("core::marker::Copy", package_graph, krate_collection);
    let ResolvedType::ResolvedPath(c) = c else {
        unreachable!()
    };
    c
}
