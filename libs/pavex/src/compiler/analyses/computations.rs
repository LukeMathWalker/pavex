use ahash::HashMap;

use crate::compiler::analyses::user_components::UserComponentId;
use crate::compiler::computation::Computation;
use crate::compiler::interner::Interner;
use crate::compiler::resolvers::{resolve_callable, CallableResolutionError};
use crate::language::{Callable, ResolvedPath};
use crate::rustdoc::CrateCollection;

pub(crate) type ComputationId = la_arena::Idx<Computation<'static>>;

#[derive(Debug)]
pub struct ComputationDb {
    interner: Interner<Computation<'static>>,
    component_id2callable_id: HashMap<UserComponentId, ComputationId>,
}

impl ComputationDb {
    /// Initialize a new (empty) computation database.
    pub fn new() -> Self {
        Self {
            interner: Interner::new(),
            component_id2callable_id: Default::default(),
        }
    }

    /// Try to resolve a callable from a resolved path.
    /// Returns the callable's id in the interner if it succeeds, an error otherwise.
    pub(crate) fn resolve_and_intern(
        &mut self,
        krate_collection: &CrateCollection,
        resolved_path: &ResolvedPath,
        user_component_id: Option<UserComponentId>,
    ) -> Result<ComputationId, CallableResolutionError> {
        let callable = resolve_callable(krate_collection, resolved_path)?;
        let callable_id = self.interner.get_or_intern(callable.into());
        if let Some(raw_user_id) = user_component_id {
            self.component_id2callable_id
                .insert(raw_user_id, callable_id);
        }
        Ok(callable_id)
    }

    /// Retrieve the id for a computation from the interner, or insert it if it doesn't exist.
    pub(crate) fn get_or_intern(
        &mut self,
        computation: impl Into<Computation<'static>>,
    ) -> ComputationId {
        self.interner.get_or_intern(computation.into())
    }
}

impl std::ops::Index<ComputationId> for ComputationDb {
    type Output = Computation<'static>;

    fn index(&self, index: ComputationId) -> &Self::Output {
        &self.interner[index]
    }
}

impl std::ops::Index<UserComponentId> for ComputationDb {
    type Output = Callable;

    fn index(&self, index: UserComponentId) -> &Self::Output {
        match &self[self.component_id2callable_id[&index]] {
            Computation::Callable(c) => c,
            n => {
                dbg!(n);
                unreachable!()
            }
        }
    }
}
