use ahash::HashMap;

use crate::compiler::analyses::user_components::UserComponentId;
use crate::compiler::component::StateInput;
use crate::compiler::interner::Interner;
use crate::language::ResolvedType;

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

    /// Retrieve the id for an input from the interner, or insert it if it doesn't exist.
    pub(crate) fn get_or_intern(&mut self, ty: StateInput) -> StateInputId {
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
