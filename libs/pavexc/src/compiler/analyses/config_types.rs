use ahash::HashMap;

use crate::compiler::analyses::user_components::UserComponentId;
use crate::compiler::component::ConfigType;
use crate::compiler::interner::Interner;

pub(crate) type ConfigTypeId = la_arena::Idx<ConfigType>;

#[derive(Debug)]
/// All the types that belong to the app configuration.
/// The primary objectives of this database:
/// - Assigning a unique id to each config type;
/// - Allow us to go back from the config type to the user-registered type it was derived from, if any;
/// - Save memory by keeping a single copy of each type in a central location, instead of
///   duplicating them everywhere.
///
/// This database is a "data bag"—it doesn't have any special logic, it just stores data.
pub struct ConfigTypeDb {
    interner: Interner<ConfigType>,
    component_id2type_id: HashMap<UserComponentId, ConfigTypeId>,
}

impl ConfigTypeDb {
    /// Initialize a new (empty) computation database.
    pub fn new() -> Self {
        Self {
            interner: Interner::new(),
            component_id2type_id: Default::default(),
        }
    }

    /// Retrieve the id for an input from the interner, or insert it if it doesn't exist.
    pub(crate) fn get_or_intern(&mut self, ty: ConfigType, id: UserComponentId) -> ConfigTypeId {
        let input_id = self.interner.get_or_intern(ty);
        self.component_id2type_id.insert(id, input_id);
        input_id
    }
}

impl std::ops::Index<ConfigTypeId> for ConfigTypeDb {
    type Output = ConfigType;

    fn index(&self, index: ConfigTypeId) -> &Self::Output {
        &self.interner[index]
    }
}

impl std::ops::Index<UserComponentId> for ConfigTypeDb {
    type Output = ConfigType;

    fn index(&self, index: UserComponentId) -> &Self::Output {
        &self[self.component_id2type_id[&index]]
    }
}
