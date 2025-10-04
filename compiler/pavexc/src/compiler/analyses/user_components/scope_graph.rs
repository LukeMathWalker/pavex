use std::collections::BTreeSet;

use ahash::{HashMap, HashMapExt};
use petgraph::algo::has_path_connecting;
use petgraph::graphmap::DiGraphMap;
use petgraph::visit::{IntoNodeIdentifiers, Reversed};

use pavex_bp_schema::Location;

/// Assign a unique ID to each *scope*.
///
/// All components are assigned to a scope.
/// Scopes can be user-defined (e.g. a `nest` call) or implicit (e.g. the root scope,
/// the scope for each request handler/middleware and the scope for the application state).
///
/// "Normal" scopes have a single parent scope and zero or more child scopes.
/// The root scope has no parent scope and zero or more child scopes.
/// The application state scope has multiple parent scopes and no child scopes.
/// Each request handler/middleware has a dedicated scope with no children—this allows us to register
/// components that are only visible to the call graph of a specific request handler/middleware.
///
/// All the components in a scope are visible to all the components in the scope's subgraph.
///
/// ## Example
///
/// ```text
///                  +---------------------+
///                  |        Root         |
///                  +---------------------+
///                             |
///                 +-----------+-----------+
///                 |                       |
///            +----+-----+           +-----+-----+
///            |  Scope 1 |           |  Scope 2  |
///            +----------+           +-----------+
///           /         |              |           \
/// +------+------+     |              |      +------+------+
/// |  RH Scope 1 |     |              |      |  RH Scope 2 |
/// +-------------+     |              |      +-------------+
///                     |              |
///                  +---------------------+
///                  |     Application     |
///                  |       State         |
///                  +---------------------+
/// ```
///
/// where `RH Scope 1` and `RH Scope 2` are request handler scopes.
#[derive(Debug, Clone)]
pub struct ScopeGraph {
    root: ScopeId,
    application_state: ScopeId,
    graph: DiGraphMap<usize, ()>,
    id2locations: HashMap<usize, Location>,
}

#[derive(Debug, Clone)]
pub struct ScopeGraphBuilder {
    root: ScopeId,
    graph: DiGraphMap<usize, ()>,
    id2locations: HashMap<usize, Location>,
    next_node_id: usize,
}

#[derive(Copy, Debug, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
/// The unique ID of a scope.
///
/// See [`ScopeGraph`] for more information.
pub struct ScopeId(usize);

impl std::fmt::Display for ScopeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Scope {}", self.0)
    }
}

impl PartialEq<ScopeId> for &ScopeId {
    fn eq(&self, other: &ScopeId) -> bool {
        self.0 == other.0
    }
}

impl ScopeId {
    /// The id of the root scope.
    pub const ROOT: ScopeId = ScopeId(0);

    /// Return `true` if the `other` scope is a parent of this scope (either directly or
    /// indirectly) or if the two scopes are equal.
    pub fn is_descendant_of(&self, other: ScopeId, scope_graph: &ScopeGraph) -> bool {
        use petgraph::visit::{Dfs, Walker};

        Dfs::new(Reversed(&scope_graph.graph), self.0)
            .iter(Reversed(&scope_graph.graph))
            .any(|node_index| node_index == other.0)
    }

    /// Return the IDs of the scopes that are direct parents of this scope, if any.
    ///
    /// E.g. if this scope is `RH Scope 1` in the example in [`ScopeGraph`], this method will return
    /// `Scope 1`, but it won't return `Root`.
    pub fn direct_parent_ids(&self, scope_graph: &ScopeGraph) -> BTreeSet<ScopeId> {
        scope_graph
            .graph
            .neighbors_directed(self.0, petgraph::Direction::Incoming)
            .map(ScopeId)
            .collect()
    }

    /// Return the IDs of the scopes that are direct children of this scope, if any.
    ///
    /// E.g. if this scope is `Root` in the example in [`ScopeGraph`], this method will return
    /// `Scope 1` and `Scope 2`, but it won't return `RH Scope 1`.
    pub fn direct_children_ids(&self, scope_graph: &ScopeGraph) -> BTreeSet<ScopeId> {
        scope_graph
            .graph
            .neighbors_directed(self.0, petgraph::Direction::Outgoing)
            .map(ScopeId)
            .collect()
    }
}

impl ScopeGraphBuilder {
    /// Create a new scope graph with a single root scope.
    fn new(root_bp_location: Location) -> Self {
        let mut graph = DiGraphMap::new();
        let root_id = graph.add_node(ScopeId::ROOT.0);
        let id2locations = {
            let mut id2locations = HashMap::new();
            id2locations.insert(root_id, root_bp_location);
            id2locations
        };
        Self {
            root: ScopeId(root_id),
            graph,
            id2locations,
            next_node_id: 1,
        }
    }

    /// Return the ID of the root scope.
    pub fn root_scope_id(&self) -> ScopeId {
        self.root
    }

    /// Add a new scope as a child of the specified parent scope.
    ///
    /// If the scope is user-defined (e.g. a `nest` or `nest_at` call), the location of the scope is also
    /// specified.
    pub fn add_scope(&mut self, parent_scope_id: ScopeId, location: Option<Location>) -> ScopeId {
        let id = {
            let id = self.next_node_id;
            self.next_node_id += 1;
            id
        };
        self.graph.add_node(id);
        self.graph.add_edge(parent_scope_id.0, id, ());
        if let Some(location) = location {
            self.id2locations.insert(id, location);
        }
        ScopeId(id)
    }

    /// Finalize the scope graph and return it.
    /// This method consumes the builder.
    ///
    /// The scope graph is immutable after this point—you won't be able to add more scopes.
    ///
    /// It also takes care of adding the application state scope as a child of all the parent nodes of "leaf" scopes.
    pub fn build(self) -> ScopeGraph {
        let mut graph = self.graph;
        let application_state = self.next_node_id;

        // Add application state scope as a child of all the *parents* of "leaf" scopes.
        let leaf_parent_ids = graph
            .node_identifiers()
            .filter(|id| {
                graph
                    .neighbors_directed(*id, petgraph::Direction::Outgoing)
                    .count()
                    == 0
            })
            .flat_map(|leaf_id| graph.neighbors_directed(leaf_id, petgraph::Direction::Incoming))
            .collect::<BTreeSet<_>>();

        graph.add_node(application_state);

        for id in leaf_parent_ids {
            graph.add_edge(id, application_state, ());
        }
        ScopeGraph {
            root: self.root,
            application_state: ScopeId(application_state),
            graph,
            id2locations: self.id2locations,
        }
    }
}

impl ScopeGraph {
    /// Start building a new scope graph.
    ///
    /// It immediately initializes the root scope, associating it with the provided location.
    pub fn builder(root_bp_location: Location) -> ScopeGraphBuilder {
        ScopeGraphBuilder::new(root_bp_location)
    }

    /// Return the ID of the root scope.
    pub fn root_scope_id(&self) -> ScopeId {
        self.root
    }

    /// Return the ID of the application state scope.
    pub fn application_state_scope_id(&self) -> ScopeId {
        self.application_state
    }

    /// Return the location of the specified scope, if it is user-defined.
    ///
    /// The only scopes that are **not** user-defined are the scopes for request handlers and the
    /// application state scope.
    pub fn get_location(&self, scope_id: ScopeId) -> Option<Location> {
        self.id2locations.get(&scope_id.0).cloned()
    }

    /// Return the ID of a scope that is a parent (either directly or transitively) of
    /// all the specified [`ScopeId`]s.
    ///
    /// There is **always** a common ancestor, since the scope graph is a directed acyclic graph
    /// rooted in the root scope.
    ///
    /// # Panics
    ///
    /// Panics if `scope_ids` is empty.
    pub fn find_common_ancestor(&self, scope_ids: Vec<ScopeId>) -> ScopeId {
        assert!(!scope_ids.is_empty());
        let mut common_ancestor = scope_ids[0];
        let mut uncovered_scope_ids = scope_ids;

        while let Some(scope_id) = uncovered_scope_ids.pop() {
            if !has_path_connecting(&self.graph, common_ancestor.0, scope_id.0, None) {
                common_ancestor = self
                    .graph
                    .neighbors_directed(common_ancestor.0, petgraph::Direction::Incoming)
                    .next()
                    .map(ScopeId)
                    .unwrap();
                // If we've reached the root, we're done.
                if common_ancestor == self.root {
                    return common_ancestor;
                }
                uncovered_scope_ids.push(scope_id);
            }
        }

        common_ancestor
    }

    /// Print a .dot representation of the scope graph, for debugging purposes.
    #[allow(unused)]
    pub fn debug_print(&self) {
        eprintln!(
            "{:?}",
            petgraph::dot::Dot::with_config(&self.graph, &[petgraph::dot::Config::EdgeNoLabel])
        )
    }
}
