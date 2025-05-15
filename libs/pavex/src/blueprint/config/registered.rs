use pavex_bp_schema::{Blueprint as BlueprintSchema, Component, ConfigType};

use crate::blueprint::{constructor::CloningStrategy, conversions::cloning2cloning};

/// The type returned by [`Blueprint::config`].
///
/// It allows you to further configure the behaviour of the registered config type.
///
/// [`Blueprint::config`]: crate::blueprint::Blueprint::config
pub struct RegisteredConfigType<'a> {
    #[allow(unused)]
    pub(crate) blueprint: &'a mut BlueprintSchema,
    /// The index of the registered config type in the blueprint's `components` vector.
    #[allow(unused)]
    pub(crate) component_id: usize,
}

impl RegisteredConfigType<'_> {
    /// Set the cloning strategy for this configuration type.
    ///
    /// By default, Pavex will clone a configuration type if it's necessary to generate code
    /// that satisfies Rust's borrow checker.
    /// You can change the default behaviour by invoking [`never_clone`](Self::never_clone).
    ///
    /// Regardless of the chosen strategy, configuration types *must* implement `Clone`,
    /// since the code-generated `ApplicationConfig` type will need to derive it.
    pub fn cloning(mut self, strategy: CloningStrategy) -> Self {
        self.config().cloning_strategy = Some(cloning2cloning(strategy));
        self
    }

    /// Set the cloning strategy to [`CloningStrategy::CloneIfNecessary`].
    /// Check out [`cloning`](Self::cloning) method for more details.
    pub fn clone_if_necessary(self) -> Self {
        self.cloning(CloningStrategy::CloneIfNecessary)
    }

    /// Set the cloning strategy to [`CloningStrategy::NeverClone`].
    /// Check out [`cloning`](Self::cloning) method for more details.
    pub fn never_clone(self) -> Self {
        self.cloning(CloningStrategy::NeverClone)
    }

    /// Use the default configuration values returned by [`Default::default`]
    /// if the user has not specified its own configuration for this type.
    ///
    /// # Requirements
    ///
    /// The configuration type *must* implement the [`Default`] trait
    /// to support this option.
    ///
    /// # Implementation notes
    ///
    /// `default_if_missing` adds a `#[serde(default)]` attribute on the corresponding
    /// configuration key in the code-generated `ApplicationConfig` struct.
    pub fn default_if_missing(mut self) -> Self {
        self.config().default_if_missing = Some(true);
        self
    }

    /// Include this configuration entry in the generated `ApplicationConfig` struct
    /// even if the type is never used by the application.
    pub fn include_if_unused(mut self) -> Self {
        self.config().include_if_unused = Some(true);
        self
    }

    fn config(&mut self) -> &mut ConfigType {
        let component = &mut self.blueprint.components[self.component_id];
        let Component::ConfigType(s) = component else {
            unreachable!("The component should be a config type")
        };
        s
    }
}
