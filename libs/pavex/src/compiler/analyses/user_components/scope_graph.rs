use indexmap::IndexSet;
use petgraph::algo::has_path_connecting;
use petgraph::graphmap::DiGraphMap;
use petgraph::visit::IntoNodeIdentifiers;

/// Assign a unique ID to each *scope*.
///
/// All components are assigned to a scope.
/// Scopes can be user-defined (e.g. a `nest` call) or implicit (e.g. the root scope,
/// the scope for each request handler and the scope for the application state).
///
/// "Normal" scopes have a single parent scope and zero or more child scopes.
/// The root scope has no parent scope and zero or more child scopes.
/// The application state scope has multiple parent scopes and no child scopes.
/// Each request handler has a dedicated scope with the application state scope as its only child.
///
/// All the components in a scope are visible to all the components in the scope's subgraph.
///
/// ## Example
///
/// ```text
///       +---------------------+
///       |        Root         |
///       +---------------------+
///                  |
///      +-----------+-----------+
///      |                       |
/// +----+-----+           +-----+-----+
/// |  Scope 1 |           |  Scope 2 |
/// +----------+           +----------+
///      |                       |
/// +----+--------+      +-----+-------+
/// |  RH Scope 1 |      |  RH Scope 2 |
/// +-------------+      +-------------+
///      |                     |
///      +---------------------+
///      |     Application     |
///      |       State         |
///      +---------------------+
/// ```
///
/// where `RH Scope 1` and `RH Scope 2` are the request handler scopes.
#[derive(Debug, Clone)]
pub struct ScopeGraph {
    root: ScopeId,
    application_state: ScopeId,
    graph: DiGraphMap<usize, ()>,
}

#[derive(Debug, Clone)]
pub struct ScopeGraphBuilder {
    root: ScopeId,
    graph: DiGraphMap<usize, ()>,
    next_node_id: usize,
}

#[derive(Copy, Debug, Clone, Hash, Eq, PartialEq)]
/// The unique ID of a scope.
///
/// See [`ScopeGraph`] for more information.
pub struct ScopeId(usize);

impl PartialEq<ScopeId> for &ScopeId {
    fn eq(&self, other: &ScopeId) -> bool {
        self.0 == other.0
    }
}

impl ScopeId {
    /// Return `true` if the `other` scope is a parent of this scope or if the two scopes are equal.
    pub fn is_child_of(&self, other: ScopeId, scope_graph: &ScopeGraph) -> bool {
        self == other
            || scope_graph
                .graph
                .neighbors_directed(other.0, petgraph::Direction::Outgoing)
                .any(|id| id == self.0)
    }

    /// Return the IDs of the parent scopes, if any.
    pub fn parent_ids(&self, scope_graph: &ScopeGraph) -> IndexSet<ScopeId> {
        scope_graph
            .graph
            .neighbors_directed(self.0, petgraph::Direction::Incoming)
            .map(ScopeId)
            .collect()
    }
}

impl ScopeGraphBuilder {
    /// Create a new scope graph with a single root scope.
    fn new() -> Self {
        let mut graph = DiGraphMap::new();
        let root_id = graph.add_node(0);
        Self {
            root: ScopeId(root_id),
            graph,
            next_node_id: 1,
        }
    }

    /// Return the ID of the root scope.
    pub fn root_scope_id(&self) -> ScopeId {
        self.root
    }

    /// Add a new scope as a child of the specified parent scope.
    pub fn add_scope(&mut self, parent_scope_id: ScopeId) -> ScopeId {
        let id = {
            let id = self.next_node_id;
            self.next_node_id += 1;
            id
        };
        self.graph.add_node(id);
        self.graph.add_edge(parent_scope_id.0, id, ());
        ScopeId(id)
    }

    /// Finalize the scope graph and return it.
    /// This method consumes the builder.
    ///
    /// The scope graph is immutable after this point—you won't be able to add more scopes.
    ///
    /// It also takes care of adding the application state scope as a child of all the "leaf" scopes.
    pub fn build(self) -> ScopeGraph {
        let mut graph = self.graph;
        let application_state = self.next_node_id;

        // Add application state scope as a child of all the "leaf" scopes.
        let leaf_scopes = graph
            .node_identifiers()
            .filter(|id| {
                graph
                    .neighbors_directed(*id, petgraph::Direction::Outgoing)
                    .count()
                    == 0
            })
            .collect::<IndexSet<_>>();
        graph.add_node(application_state);
        for id in leaf_scopes {
            graph.add_edge(id, application_state, ());
        }
        ScopeGraph {
            root: self.root,
            application_state: ScopeId(application_state),
            graph,
        }
    }
}

impl ScopeGraph {
    /// Start building a new scope graph.
    /// It immediately initializes the root scope.
    pub fn builder() -> ScopeGraphBuilder {
        ScopeGraphBuilder::new()
    }

    /// Return the ID of the root scope.
    pub fn root_scope_id(&self) -> ScopeId {
        self.root
    }

    /// Return the ID of the application state scope.
    pub fn application_state_scope_id(&self) -> ScopeId {
        self.application_state
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
        assert!(scope_ids.len() > 0);
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
}
