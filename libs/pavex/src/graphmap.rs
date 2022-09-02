use bimap::BiHashMap;
use petgraph::data::Build;
use petgraph::graphmap::DiGraphMap;
use std::default::Default;
use std::hash::Hash;
use std::ops::Index;

/// A wrapper around `DiGraphMap` that allows using a non-Copy type as node type (`N`).
///
/// The node data is stored in a bidirectional hashmap that associates each payload with a unique
/// index integer.
#[derive(Debug, Clone)]
pub struct GraphMap<N: Hash + Eq> {
    pub graph: DiGraphMap<u32, ()>,
    node_data: BiHashMap<N, u32>,
    cursor: u32,
}

impl<N: Hash + Eq> Default for GraphMap<N> {
    fn default() -> Self {
        Self {
            graph: Default::default(),
            node_data: BiHashMap::new(),
            cursor: 0,
        }
    }
}

impl<N: Hash + Eq> GraphMap<N> {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn add_node(&mut self, node: N) -> u32 {
        if let Some(index) = self.node_data.get_by_left(&node) {
            *index
        } else {
            let index = self.cursor;
            self.cursor += 1;
            self.node_data.insert(node, index);
            index
        }
    }

    #[allow(dead_code)]
    pub fn get_node_index(&self, node_data: &N) -> Option<u32> {
        self.node_data.get_by_left(node_data).cloned()
    }

    pub fn update_edge(&mut self, source_node_index: u32, destination_node_index: u32) {
        self.graph
            .update_edge(source_node_index, destination_node_index, ());
    }
}

impl<N: Hash + Eq> Index<u32> for GraphMap<N> {
    type Output = N;

    fn index(&self, index: u32) -> &Self::Output {
        self.node_data.get_by_right(&index).unwrap()
    }
}
