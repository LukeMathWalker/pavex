//! Utilities to load the hierarchical configuration for a Pavex application.
//!
//! [`ConfigLoader`] is the key type in this module.
//!
//! # Guide
//!
//! Check out [the guide](https://pavex.dev/docs/guide/configuration/)
//! for a thorough introduction to Pavex configuration system.
use std::{path::PathBuf, str::FromStr};

use anyhow::Context;
use figment::{
    Figment,
    providers::{Env, Format, Yaml},
};
use serde::de::DeserializeOwned;

#[derive(Clone, Debug)]
/// A utility to load hierarchical configuration in a Pavex application.
///
/// Check out [`ConfigLoader::load`] for more information.
///
/// # Example
///
/// ```rust,no_run
/// use pavex::config::{ConfigLoader, ConfigProfile};
///
/// #[derive(ConfigProfile, Debug, Clone, Copy, PartialEq, Eq)]
/// pub enum Profile {
///     #[px(profile = "dev")]
///     Development,
///     #[px(profile = "prod")]
///     Production,
/// }
///
/// #[derive(Debug, Clone, serde::Deserialize)]
/// pub struct Config {
///     database_url: String,
///     // Other fields...
/// }
///
/// # fn main() -> anyhow::Result<()> {
/// let config: Config = ConfigLoader::<Profile>::new().load()?;
/// # Ok(())
/// # }
/// ```
pub struct ConfigLoader<Profile> {
    configuration_dir: Option<PathBuf>,
    profile: Option<Profile>,
}

/// A macro to derive an implementation of the [`ConfigProfile`] trait.
///
/// ```rust
/// use pavex::config::ConfigProfile;
///
/// #[derive(ConfigProfile)]
/// pub enum Profile {
///     Development, // "development"
///     Production,  // "production"
/// }
/// ```
///
/// ## Usage
///
/// By default, each variant is converted to a **snake_case** string representation:
///
/// ```rust
/// use pavex::config::ConfigProfile;
/// use std::str::FromStr;
///
/// #[derive(ConfigProfile)]
/// pub enum Profile {
///     LocalDevelopment, // "local_development"
///     Production,  // "production"
/// }
///
/// # fn main() {
/// let p = Profile::from_str("local_development").unwrap();
/// assert_eq!(p.as_ref(), "local_development");
/// # }
/// ```
///
/// ## Custom Profile Names
///
/// You can override the default representation using `#[px(profile = "...")]`:
///
/// ```rust
/// use pavex::config::ConfigProfile;
/// use std::str::FromStr;
///
/// #[derive(ConfigProfile)]
/// pub enum Profile {
///     #[px(profile = "dev")]
///     Development,
///     #[px(profile = "prod")]
///     Production,
/// }
///
/// # fn main() -> anyhow::Result<()> {
/// let p = Profile::from_str("dev")?;
/// assert_eq!(p.as_ref(), "dev");
/// # Ok(())
/// # }
/// ```
///
///
/// ## Limitations
///
/// The macro only works on enums with unit variants.
/// Enums with fields, structs, or unions are not supported and will result in a compile-time error.
///
/// If you need more flexibility, consider implementing [`ConfigProfile`] manually.
pub use pavex_macros::ConfigProfile;

/// Configuration profiles are used by Pavex applications to determine
/// which configuration file to load.
///
/// They are usually modeled as an enum with unit variants, one for each profile.
///
/// ## Deriving an implementation
///
/// You can automatically derive an implementation of `ConfigProfile` using the `#[derive(ConfigProfile)]` attribute.
///
/// ```rust
/// use pavex::config::ConfigProfile;
///
/// // Equivalent to the manual implementation below!
/// #[derive(ConfigProfile, Debug, Clone, Copy, PartialEq, Eq)]
/// pub enum Profile {
///     #[px(profile = "dev")]
///     Development,
///     #[px(profile = "prod")]
///     Production,
/// }
/// ```
///
/// Check out the [macro documentation](derive.ConfigProfile.html) for more details.
///
/// ## Manual implementation
///
/// If you need more flexibility, you can implement `ConfigProfile` manually:
///
/// ```rust
/// use pavex::config::ConfigProfile;
///
/// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// pub enum Profile {
///     Development,
///     Production,
/// }
///
/// impl std::str::FromStr for Profile {
///     type Err = anyhow::Error;
///
///     fn from_str(s: &str) -> Result<Self, Self::Err> {
///         match s {
///             "dev" => Ok(Profile::Development),
///             "prod" => Ok(Profile::Production),
///             _ => anyhow::bail!("Unknown profile: {}", s),
///         }
///     }
/// }
///
/// impl AsRef<str> for Profile {
///     fn as_ref(&self) -> &str {
///         match self {
///             Profile::Development => "dev",
///             Profile::Production => "prod",
///         }
///     }
/// }
///
/// impl ConfigProfile for Profile {}
/// ```
///
/// The value returned by `as_ref()` is used as the name of the profile-specific configuration file.
pub trait ConfigProfile:
    FromStr<Err: std::fmt::Display + std::fmt::Debug + Send + Sync + 'static> + AsRef<str>
{
    /// Load and parse the configuration profile out of the `PX_PROFILE` environment variable.
    fn load() -> Result<Self, errors::ConfigProfileLoadError> {
        let profile = std::env::var(PROFILE_ENV_VAR).context(
            "Failed to load the configuration profile: the environment variable `PX_PROFILE` is either not set or set to a value that contains invalid UTF-8"
        ).map_err(errors::ConfigProfileLoadError)?;
        Self::from_str(&profile).map_err(|e| anyhow::anyhow!(e).context(
            "Failed to parse the configuration profile from the `{PROFILE_ENV_VAR}` environment variable",
        ))
        .map_err(errors::ConfigProfileLoadError)
    }
}

static PROFILE_ENV_VAR: &str = "PX_PROFILE";

impl<Profile> ConfigLoader<Profile>
where
    Profile: ConfigProfile,
{
    /// Initialize a new [`ConfigLoader`] instance.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            configuration_dir: None,
            profile: None,
        }
    }

    /// Specify the application profile manually, rather than loading it
    /// from the `PX_PROFILE` environment variable.
    pub fn profile(mut self, profile: Profile) -> Self {
        self.profile = Some(profile);
        self
    }

    /// Specify the path to the directory where configuration files are stored.
    ///
    /// # Relative paths
    ///
    /// If you provide a relative path, it will be resolved relative to the current working directory.
    /// If it's not found there, Pavex will look for it in the parent directory, up until the root directory,
    /// stopping at the first hit.
    ///
    /// # Absolute paths
    ///
    /// If you provide an absolute path, it will be used as is.
    ///
    /// # Default value
    ///
    /// By default, Pavex looks for configuration files under `configuration/`.
    pub fn configuration_dir<Dir>(mut self, dir: Dir) -> Self
    where
        Dir: Into<PathBuf>,
    {
        self.configuration_dir = Some(dir.into());
        self
    }

    /// Load the configuration for the application by merging together three sources:
    ///
    /// 1. Environment variables (`PX_*`)
    /// 2. Profile-specific configuration file (`{configuration_dir}/{profile}.yml`)
    /// 3. Base configuration file (`{configuration_dir}/base.yml`)
    ///
    /// The list above is ordered by precedence: environment variables take precedence
    /// over profile-specific configuration files, which in turn take precedence
    /// over the base configuration file.
    ///
    /// # Guide
    ///
    /// Check out [the guide](https://pavex.dev/docs/guide/configuration/loading/)
    /// for an overview of Pavex's configuration hierarchy, as well as a detailed
    /// explanation of the naming convention used for environment variables.
    pub fn load<Config>(self) -> Result<Config, errors::ConfigLoadError>
    where
        Config: DeserializeOwned,
    {
        let profile = match self.profile {
            Some(profile) => profile,
            None => Profile::load().map_err(|e| errors::ConfigLoadError(e.into()))?,
        };
        let configuration_dir = self
            .configuration_dir
            .unwrap_or_else(|| PathBuf::from("configuration"));
        let span = tracing::info_span!(
            "Loading configuration",
            configuration.directory = %configuration_dir.display(),
            configuration.profile = %profile.as_ref(),
        );
        let _guard = span.enter();
        // Load configuration files from the specified directory
        // and return a `Config` instance.
        let base_filepath = configuration_dir.join("base.yml");
        let profile_filepath = configuration_dir.join(format!("{}.yml", profile.as_ref()));

        let prefix = "PX_";
        let env_source = Env::prefixed(prefix)
            .split("__")
            // We explicitly filter out the `PX_PROFILE` environment variable
            // to allow users to set `#[serde(deny_unknown_fields)]` on their configuration type.
            // Without this `ignore`, `serde` would complain about `PX_PROFILE` being unknown.
            .ignore(&[PROFILE_ENV_VAR.strip_prefix(prefix).unwrap()]);
        let figment = Figment::new()
            .merge(Yaml::file(base_filepath))
            .merge(Yaml::file(profile_filepath))
            .merge(env_source);

        let configuration: Config = figment
            .extract()
            .context("Failed to load hierarchical configuration")
            .map_err(errors::ConfigLoadError)?;
        Ok(configuration)
    }
}

/// Errors that can occur when loading configuration.
pub mod errors {
    #[derive(Debug, thiserror::Error)]
    #[error("Failed to load configuration")]
    /// The error returned by [`ConfigLoader::load`](super::ConfigLoader::load).
    pub struct ConfigLoadError(#[source] pub(super) anyhow::Error);

    #[derive(Debug, thiserror::Error)]
    #[error(transparent)]
    /// The error returned by [`ConfigProfile::load`](super::ConfigProfile::load).
    pub struct ConfigProfileLoadError(pub(super) anyhow::Error);
}
