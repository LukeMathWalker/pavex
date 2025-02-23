use pavex_bp_schema::Type;

use crate::blueprint::Blueprint;
use crate::blueprint::constructor::CloningStrategy;
use crate::blueprint::conversions::raw_identifiers2type;
use crate::blueprint::prebuilt::RegisteredPrebuiltType;
use crate::blueprint::reflection::RawIdentifiers;

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
/// [`PrebuiltType`] is primarily used by kits to allow users to customize (or disable!)
/// state inputs **before** registering them with a [`Blueprint`].
#[derive(Clone, Debug)]
pub struct PrebuiltType {
    pub(in crate::blueprint) type_: Type,
    pub(in crate::blueprint) cloning_strategy: Option<CloningStrategy>,
}

impl PrebuiltType {
    /// Create a new (unregistered) state input.
    ///
    /// Check out the documentation of [`Blueprint::prebuilt`] for more details
    /// on state inputs.
    #[track_caller]
    pub fn new(type_: RawIdentifiers) -> Self {
        Self {
            type_: raw_identifiers2type(type_),
            cloning_strategy: None,
        }
    }

    /// Set the cloning strategy for the state input type.
    ///
    /// Check out the documentation of [`CloningStrategy`] for more details.
    pub fn cloning(mut self, cloning_strategy: CloningStrategy) -> Self {
        self.cloning_strategy = Some(cloning_strategy);
        self
    }

    /// Set the cloning strategy to [`CloningStrategy::CloneIfNecessary`].  
    /// Check out [`PrebuiltType::cloning`] for more details.
    pub fn clone_if_necessary(self) -> Self {
        self.cloning(CloningStrategy::CloneIfNecessary)
    }

    /// Set the cloning strategy to [`CloningStrategy::NeverClone`].  
    /// Check out [`PrebuiltType::cloning`] for more details.
    pub fn never_clone(self) -> Self {
        self.cloning(CloningStrategy::NeverClone)
    }

    /// Register this state input with a [`Blueprint`].
    ///
    /// Check out the documentation of [`Blueprint::prebuilt`] for more details.
    pub fn register(self, bp: &mut Blueprint) -> RegisteredPrebuiltType {
        bp.register_prebuilt_type(self)
    }
}
