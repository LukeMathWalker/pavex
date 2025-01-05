use pavex_bp_schema::{Blueprint as BlueprintSchema, Component, PrebuiltType};

use crate::blueprint::{constructor::CloningStrategy, conversions::cloning2cloning};

/// The type returned by [`Blueprint::prebuilt`].
///
/// It allows you to further configure the behaviour of the registered prebuilt type.
///
/// [`Blueprint::prebuilt`]: crate::blueprint::Blueprint::prebuilt
pub struct RegisteredPrebuiltType<'a> {
    #[allow(unused)]
    pub(crate) blueprint: &'a mut BlueprintSchema,
    /// The index of the registered state input in the blueprint's `components` vector.
    #[allow(unused)]
    pub(crate) component_id: usize,
}

impl RegisteredPrebuiltType<'_> {
    /// Set the cloning strategy for the output type returned by this constructor.
    ///
    /// By default,
    /// Pavex will **never** try to clone the output type returned by a constructor.  
    /// If the output type implements [`Clone`], you can change the default by setting the cloning strategy
    /// to [`CloningStrategy::CloneIfNecessary`]: Pavex will clone the output type if
    /// it's necessary to generate code that satisfies Rust's borrow checker.
    pub fn cloning(mut self, strategy: CloningStrategy) -> Self {
        self.prebuilt().cloning_strategy = Some(cloning2cloning(strategy));
        self
    }

    /// Set the cloning strategy to [`CloningStrategy::CloneIfNecessary`].  
    /// Check out [`RegisteredPrebuiltType::cloning`] for more details.
    pub fn clone_if_necessary(self) -> Self {
        self.cloning(CloningStrategy::CloneIfNecessary)
    }

    /// Set the cloning strategy to [`CloningStrategy::NeverClone`].  
    /// Check out [`RegisteredPrebuiltType::cloning`] for more details.
    pub fn never_clone(self) -> Self {
        self.cloning(CloningStrategy::NeverClone)
    }

    fn prebuilt(&mut self) -> &mut PrebuiltType {
        let component = &mut self.blueprint.components[self.component_id];
        let Component::PrebuiltType(s) = component else {
            unreachable!("The component should be a state input")
        };
        s
    }
}
