use pavex_bp_schema::Type;

use crate::blueprint::conversions::raw_identifiers2type;
use crate::blueprint::reflection::RawIdentifiers;
use crate::blueprint::state::RegisteredStateInput;
use crate::blueprint::Blueprint;

/// A state input that has been configured but has not yet been registered with a [`Blueprint`].
///
/// # Guide
///
/// Check out the ["Dependency injection"](https://pavex.dev/docs/guide/dependency_injection)
/// section of Pavex's guide for a thorough introduction to dependency injection
/// in Pavex applications.
///
/// # Use cases
///
/// [`StateInput`] is primarily used by kits to allow users to customize (or disable!)
/// state inputs **before** registering them with a [`Blueprint`].
#[derive(Clone, Debug)]
pub struct StateInput {
    pub(in crate::blueprint) type_: Type,
}

impl StateInput {
    /// Create a new (unregistered) state input.
    ///
    /// Check out the documentation of [`Blueprint::state_input`] for more details
    /// on state inputs.
    #[track_caller]
    pub fn new(type_: RawIdentifiers) -> Self {
        Self {
            type_: raw_identifiers2type(type_),
        }
    }

    /// Register this state input with a [`Blueprint`].
    ///
    /// Check out the documentation of [`Blueprint::state_input`] for more details.
    pub fn register(self, bp: &mut Blueprint) -> RegisteredStateInput {
        bp.register_state_input(self)
    }
}
