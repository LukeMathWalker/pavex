use super::reflection::AnnotationCoordinates;
use crate::blueprint::{CloningPolicy, conversions::cloning2cloning};
use pavex_bp_schema::{Blueprint as BlueprintSchema, Component, ConfigType};

/// The input type for [`Blueprint::config`].
///
/// Check out [`Blueprint::config`] for more information on how to manage configuration types
/// in Pavex.
///
/// # Stability guarantees
///
/// Use the [`config`](macro@crate::config) attribute macro to create instances of `Config`.\
/// `Config`'s fields are an implementation detail of Pavex's macros and should not be relied upon:
/// newer versions of Pavex may add, remove or modify its fields.
///
/// [`Blueprint::config`]: crate::Blueprint::config
pub struct Config {
    #[doc(hidden)]
    pub coordinates: AnnotationCoordinates,
}

/// A configuration type registered via [`Blueprint::config`].
///
/// # Example
///
/// You can use the methods exposed by [`RegisteredConfig`] to tune the behaviour
/// of the registered configuration type.
/// For example, instruct Pavex to use the `Default` implementation if the user configuration
/// doesn't specify a value for `pool`:
///
/// ```rust
/// use pavex::{config, Blueprint};
///
/// #[config(key = "pool")]
/// #[derive(serde::Deserialize, Debug, Clone)]
/// pub struct PoolConfig {
///     pub max_n_connections: u32,
///     pub min_n_connections: u32,
/// }
///
/// impl Default for PoolConfig {
///     fn default() -> Self {
///         Self {
///             max_n_connections: 10,
///             min_n_connections: 2,
///         }
///     }
/// }
///
/// let mut bp = Blueprint::new();
/// // This is equivalent to `#[config(key = "pool", default_if_missing)]`
/// bp.config(POOL_CONFIG).default_if_missing();
/// ```
///
/// # Example: override the annotation
///
/// You can also override the behaviour specified via the [`config`](macro@crate::config) attribute.
///
/// ```rust
/// use pavex::{config, Blueprint};
///
/// #[config(key = "pool", default_if_missing)]
/// #[derive(serde::Deserialize, Debug, Clone)]
/// pub struct PoolConfig {
///     pub max_n_connections: u32,
///     pub min_n_connections: u32,
/// }
///
/// # impl Default for PoolConfig {
/// #     fn default() -> Self {
/// #         Self {
/// #             max_n_connections: 10,
/// #             min_n_connections: 2,
/// #         }
/// #     }
/// # }
/// #
/// let mut bp = Blueprint::new();
/// // Using `required`, we are overriding the `default_if_missing` flag
/// // specified via the `config` attribute.
/// // This is equivalent to `#[config(key = "pool")]`, thus restoring
/// // the default behaviour.
/// bp.config(POOL_CONFIG).required();
/// ```
///
/// [`Blueprint::config`]: crate::Blueprint::config
pub struct RegisteredConfig<'a> {
    #[allow(unused)]
    pub(crate) blueprint: &'a mut BlueprintSchema,
    /// The index of the registered config type in the blueprint's `components` vector.
    #[allow(unused)]
    pub(crate) component_id: usize,
}

impl RegisteredConfig<'_> {
    /// Set the cloning strategy for this configuration type.
    ///
    /// By default, Pavex will clone a configuration type if it's necessary to generate code
    /// that satisfies Rust's borrow checker.
    /// You can change the default behaviour by invoking [`never_clone`](Self::never_clone).
    ///
    /// Regardless of the chosen strategy, configuration types *must* implement `Clone`,
    /// since the code-generated `ApplicationConfig` type will need to derive it.
    pub fn cloning(mut self, strategy: CloningPolicy) -> Self {
        self.config().cloning_policy = Some(cloning2cloning(strategy));
        self
    }

    /// Set the cloning strategy to [`CloningPolicy::CloneIfNecessary`].
    /// Check out [`cloning`](Self::cloning) method for more details.
    pub fn clone_if_necessary(self) -> Self {
        self.cloning(CloningPolicy::CloneIfNecessary)
    }

    /// Set the cloning strategy to [`CloningPolicy::NeverClone`].
    /// Check out [`cloning`](Self::cloning) method for more details.
    pub fn never_clone(self) -> Self {
        self.cloning(CloningPolicy::NeverClone)
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

    /// Force the user to specify a value for this configuration entry.
    ///
    /// It's the opposite of [`default_if_missing`](Self::default_if_missing).
    pub fn required(mut self) -> Self {
        self.config().default_if_missing = Some(false);
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
