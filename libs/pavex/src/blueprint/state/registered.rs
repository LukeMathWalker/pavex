use pavex_bp_schema::{Blueprint as BlueprintSchema, Component, StateInput};

use crate::blueprint::{constructor::CloningStrategy, conversions::cloning2cloning};

/// The type returned by [`Blueprint::state_input`].
///
/// It allows you to further configure the behaviour of the registered constructor.
///
/// [`Blueprint::state_input`]: crate::blueprint::Blueprint::state_input
pub struct RegisteredStateInput<'a> {
    #[allow(unused)]
    pub(crate) blueprint: &'a mut BlueprintSchema,
    /// The index of the registered state input in the blueprint's `components` vector.
    #[allow(unused)]
    pub(crate) component_id: usize,
}

impl<'a> RegisteredStateInput<'a> {
    /// Set the cloning strategy for the output type returned by this constructor.
    ///
    /// By default,
    /// Pavex will **never** try to clone the output type returned by a constructor.  
    /// If the output type implements [`Clone`], you can change the default by setting the cloning strategy
    /// to [`CloningStrategy::CloneIfNecessary`]: Pavex will clone the output type if
    /// it's necessary to generate code that satisfies Rust's borrow checker.
    pub fn cloning(mut self, strategy: CloningStrategy) -> Self {
        self.state_input().cloning_strategy = Some(cloning2cloning(strategy));
        self
    }

    fn state_input(&mut self) -> &mut StateInput {
        let component = &mut self.blueprint.components[self.component_id];
        let Component::StateInput(s) = component else {
            unreachable!("The component should be a state input")
        };
        s
    }
}
