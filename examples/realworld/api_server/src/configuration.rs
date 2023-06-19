use anyhow::Context;
use conduit_core::configuration::Config;
use figment::{
    providers::{Env, Format, Yaml},
    Figment,
};

/// Retrieve the application configuration by merging together multiple configuration sources.
///
/// # Application profiles
///
/// We use the concept of application profiles to allow for
/// different configuration values depending on the type of environment
/// the application is running in.
///
/// We don't rely on `figment`'s built-in support for profiles because
/// we want to make sure that values for different profiles are not co-located in
/// the same configuration file.  
/// This makes it easier to avoid leaking sensitive information by mistake (e.g.
/// by committing configuration values for the `dev` profile to the repository).
///
/// # Hierarchy
///
/// The configuration sources are:
///
/// 1. `base.yml` - Contains the default configuration values, common to all profiles.
/// 2. `<profile>.yml` - Contains the configuration values specific to the desired profile.
/// 3. Environment variables - Contains the configuration values specific to the current environment.
///
/// The configuration sources are listed in priority order, i.e.
/// the last source in the list will override any previous source.
///
/// For example, if the same configuration key is defined in both
/// the YAML file and the environment, the value from the environment
/// will be used.
pub fn load_configuration() -> Result<Config, anyhow::Error> {
    let application_profile =
        load_app_profile().context("Failed to load the desired application profile")?;

    let configuration_dir = {
        let manifest_dir = env!(
            "CARGO_MANIFEST_DIR",
            "`CARGO_MANIFEST_DIR` was not set. Are you using a custom build system?"
        );
        std::path::Path::new(manifest_dir).join("configuration")
    };

    let base_filepath = configuration_dir.join("base.yml");

    let profile_filename = format!("{}.yml", application_profile.as_str());
    let profile_filepath = configuration_dir.join(profile_filename);

    let figment = Figment::new()
        .merge(Yaml::file(base_filepath))
        .merge(Yaml::file(profile_filepath))
        .merge(Env::prefixed("APP_"));

    let configuration: Config = figment
        .extract()
        .context("Failed to load hierarchical configuration")?;
    Ok(configuration)
}

/// Load the application profile from the `APP_PROFILE` environment variable.
fn load_app_profile() -> Result<ApplicationProfile, anyhow::Error> {
    static PROFILE_ENV_VAR: &str = "APP_PROFILE";

    let raw_value = std::env::var(PROFILE_ENV_VAR)
        .with_context(|| format!("Failed to read the `{PROFILE_ENV_VAR}` environment variable"))?;

    raw_value
        .parse()
        .with_context(|| format!("Failed to parse the `{PROFILE_ENV_VAR}` environment variable"))
}

/// The application profile, i.e. the type of environment the application is running in.
/// See [`load_configuration`] for more details.
enum ApplicationProfile {
    /// Test profile.
    ///
    /// This is the profile used by the integration test suite.
    Test,
    /// Local development profile.
    ///
    /// This is the profile you should use when running the application locally
    /// for exploratory testing.
    ///
    /// The corresponding configuration file is `dev.yml` and it's *never* committed to the repository.
    Dev,
    /// Production profile.
    ///
    /// This is the profile you should use when running the application in production—e.g.
    /// when deploying it to a staging or production environment, exposed to live traffic.
    ///
    /// The corresponding configuration file is `prod.yml`.  
    /// It's committed to the repository, but it's meant to contain exclusively
    /// non-sensitive configuration values.
    Prod,
}

impl ApplicationProfile {
    /// Return the environment as a string.
    pub fn as_str(&self) -> &'static str {
        match self {
            ApplicationProfile::Test => "test",
            ApplicationProfile::Dev => "dev",
            ApplicationProfile::Prod => "prod",
        }
    }
}

impl std::str::FromStr for ApplicationProfile {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "test" => Ok(ApplicationProfile::Test),
            "dev" | "development" => Ok(ApplicationProfile::Dev),
            "prod" | "production" => Ok(ApplicationProfile::Prod),
            s => Err(anyhow::anyhow!(
                "`{}` is not a valid application profile.\nValid options are: `test`, `dev`, `prod`.",
                s
            )),
        }
    }
}
