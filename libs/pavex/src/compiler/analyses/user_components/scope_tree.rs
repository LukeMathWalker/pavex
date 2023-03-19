use std::borrow::Cow;

use id_tree::InsertBehavior::AsRoot;
use id_tree::{InsertBehavior, Node, NodeId, Tree};

/// Assign a unique ID to each *scope*.
///
/// All components are assigned to a scope.
/// Each scope has a single parent scope (except for the root scope) and zero or more child scopes;
/// in other words, scopes form a tree.
///
/// All the components in a scope are visible to all the components in the scope's subtree.
#[derive(Debug, Clone)]
pub struct ScopeTree {
    root: NodeId,
    tree: Tree<usize>,
    next_node_id: usize,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
/// The unique ID of a scope.
///
/// See [`ScopeTree`] for more information.
pub struct ScopeId<'a>(Cow<'a, NodeId>);

impl<'a> ScopeId<'a> {
    /// Perform a deep clone of the scope ID.
    ///
    /// It is expected to be cheap (a `ScopeId` is three integers in a trench coat
    /// and it should probably just be `Copy`, but `NodeId` isn't, so..).
    pub fn into_owned(self) -> ScopeId<'static> {
        match self.0 {
            Cow::Borrowed(b) => ScopeId(Cow::Owned(b.to_owned())),
            Cow::Owned(b) => ScopeId(Cow::Owned(b)),
        }
    }

    /// Return `true` if the `other` scope is a parent of this scope or if the two scopes are equal.
    pub fn is_child_of(&self, other: &ScopeId<'_>, scope_tree: &ScopeTree) -> bool {
        self == other
            || scope_tree
                .tree
                .children_ids(&other.0)
                .unwrap()
                .any(|id| id == self.0.as_ref())
    }
}

impl ScopeTree {
    /// Create a new scope tree with a single root scope.
    pub fn new() -> Self {
        let mut tree = Tree::new();
        let root = Node::new(0);
        let root_id = tree.insert(root, AsRoot).unwrap();
        Self {
            root: root_id,
            tree,
            next_node_id: 1,
        }
    }

    /// Return the ID of the root scope.
    pub fn root_scope_id(&self) -> ScopeId {
        ScopeId(Cow::Borrowed(&self.root))
    }

    /// Add a new scope as a child of the specified parent scope.
    pub fn add_scope(&mut self, parent_scope_id: ScopeId<'_>) -> ScopeId<'static> {
        let id = {
            let id = self.next_node_id;
            self.next_node_id += 1;
            id
        };
        let node = Node::new(id);
        let node_id = self
            .tree
            .insert(node, InsertBehavior::UnderNode(parent_scope_id.0.as_ref()))
            .unwrap();
        ScopeId(Cow::Owned(node_id))
    }
}
