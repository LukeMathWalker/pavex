use crate::compiler::analyses::computations::ComputationId;
use crate::compiler::analyses::user_components::{ScopeId, UserComponentId};

pub(crate) mod component;
pub(crate) mod db;

pub(crate) mod hydrated;
pub(crate) mod unregistered;

pub(crate) use db::{ComponentDb, ComponentId};
pub(crate) use hydrated::HydratedComponent;
pub(crate) use unregistered::UnregisteredComponent;

/// Describe the relationship between this component and one of its input parameters with
/// respect to Rust's ownership semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ConsumptionMode {
    /// The component takes the input by value, consuming it (e.g. `fn f(t: MyStruct)`).
    Move,
    /// The component takes a shared borrow of the input (e.g. `fn f(t: &MyStruct)`).
    SharedBorrow,
}

/// When should the transformer node be inserted in the graph?
#[derive(Debug, Clone, Copy, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub(crate) enum InsertTransformer {
    /// Always insert the transformer node if the transformed component appears in the graph.
    Eagerly,
    /// Don't automatically insert the transformer node. Instead, the compiler
    /// will manually insert it when it is needed.
    ///
    /// This is primarily used for cloning nodes.
    Lazily,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum SourceId {
    ComputationId(ComputationId, ScopeId),
    UserComponentId(UserComponentId),
}

impl From<UserComponentId> for SourceId {
    fn from(value: UserComponentId) -> Self {
        Self::UserComponentId(value)
    }
}
