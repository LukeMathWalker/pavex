use ahash::HashMap;

use crate::compiler::analyses::user_components::UserComponentId;
use crate::compiler::component::PrebuiltType;
use crate::compiler::interner::Interner;
use crate::language::ResolvedType;

pub(crate) type PrebuiltTypeId = la_arena::Idx<ResolvedType>;

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
pub struct PrebuiltTypeDb {
    interner: Interner<ResolvedType>,
    component_id2type_id: HashMap<UserComponentId, PrebuiltTypeId>,
}

impl PrebuiltTypeDb {
    /// Initialize a new (empty) computation database.
    pub fn new() -> Self {
        Self {
            interner: Interner::new(),
            component_id2type_id: Default::default(),
        }
    }

    /// Retrieve the id for an input from the interner, or insert it if it doesn't exist.
    pub(crate) fn get_or_intern(
        &mut self,
        ty: PrebuiltType,
        id: UserComponentId,
    ) -> PrebuiltTypeId {
        let input_id = self.interner.get_or_intern(ty.into());
        self.component_id2type_id.insert(id, input_id);
        input_id
    }
}

impl std::ops::Index<PrebuiltTypeId> for PrebuiltTypeDb {
    type Output = ResolvedType;

    fn index(&self, index: PrebuiltTypeId) -> &Self::Output {
        &self.interner[index]
    }
}

impl std::ops::Index<UserComponentId> for PrebuiltTypeDb {
    type Output = ResolvedType;

    fn index(&self, index: UserComponentId) -> &Self::Output {
        &self[self.component_id2type_id[&index]]
    }
}
