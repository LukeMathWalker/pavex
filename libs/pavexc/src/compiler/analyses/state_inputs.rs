use ahash::HashMap;

use crate::compiler::analyses::user_components::UserComponentId;
use crate::compiler::interner::Interner;
use crate::compiler::resolvers::{resolve_type_path, TypeResolutionError};
use crate::language::{ResolvedPath, ResolvedType};
use crate::rustdoc::CrateCollection;

pub(crate) type StateInputId = la_arena::Idx<ResolvedType>;

#[derive(Debug)]
/// A database of all the types that might be used as inputs to the generated constructor for
/// the application state.  
/// The primary objectives of this database:
/// - Assigning a unique id to each input type;
/// - Allow us to go back from the input type to the user-registered type it was derived from, if any;
/// - Save memory by keeping a single copy of each type in a central location, instead of
///   duplicating them everywhere.
///
/// This database is a "data bag"—it doesn't have any special logic, it just stores data.
pub struct StateInputDb {
    interner: Interner<ResolvedType>,
    component_id2type_id: HashMap<UserComponentId, StateInputId>,
}

impl StateInputDb {
    /// Initialize a new (empty) computation database.
    pub fn new() -> Self {
        Self {
            interner: Interner::new(),
            component_id2type_id: Default::default(),
        }
    }

    /// Try to resolve a type from a resolved path.
    /// Returns the type's id in the interner if it succeeds, an error otherwise.
    pub(crate) fn resolve_and_intern(
        &mut self,
        krate_collection: &CrateCollection,
        resolved_path: &ResolvedPath,
        user_component_id: Option<UserComponentId>,
    ) -> Result<StateInputId, TypeResolutionError> {
        let ty = resolve_type_path(resolved_path, krate_collection)?;
        let ty_id = self.interner.get_or_intern(ty);
        if let Some(raw_user_id) = user_component_id {
            self.component_id2type_id.insert(raw_user_id, ty_id);
        }
        Ok(ty_id)
    }

    /// Retrieve the id for a computation from the interner, or insert it if it doesn't exist.
    pub(crate) fn get_or_intern(&mut self, ty: impl Into<ResolvedType>) -> StateInputId {
        self.interner.get_or_intern(ty.into())
    }
}

impl std::ops::Index<StateInputId> for StateInputDb {
    type Output = ResolvedType;

    fn index(&self, index: StateInputId) -> &Self::Output {
        &self.interner[index]
    }
}

impl std::ops::Index<UserComponentId> for StateInputDb {
    type Output = ResolvedType;

    fn index(&self, index: UserComponentId) -> &Self::Output {
        &self[self.component_id2type_id[&index]]
    }
}
