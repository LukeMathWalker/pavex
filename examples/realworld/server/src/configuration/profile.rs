use anyhow::Context;
use std::env::VarError;

/// The application profile, i.e. the type of environment the application is running in.
/// See [`Config::load`] for more details on how it influences the way configuration
/// is loaded.
///
/// [`Config::load`]: crate::configuration::Config::load
pub enum ApplicationProfile {
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
    /// Load the application profile from the `APP_PROFILE` environment variable.
    pub fn load(
        default_profile: Option<ApplicationProfile>,
    ) -> Result<ApplicationProfile, anyhow::Error> {
        static PROFILE_ENV_VAR: &str = "APP_PROFILE";

        match std::env::var(PROFILE_ENV_VAR) {
            Ok(raw_value) => raw_value.parse().with_context(|| {
                format!("Failed to parse the `{PROFILE_ENV_VAR}` environment variable")
            }),
            Err(VarError::NotPresent) if default_profile.is_some() => Ok(default_profile.unwrap()),
            Err(e) => Err(anyhow::anyhow!(e).context(format!(
                "Failed to read the `{PROFILE_ENV_VAR}` environment variable"
            ))),
        }
    }

    /// Return the environment as a string.
    pub fn as_str(&self) -> &'static str {
        match self {
            ApplicationProfile::Dev => "dev",
            ApplicationProfile::Prod => "prod",
        }
    }
}

impl std::str::FromStr for ApplicationProfile {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "dev" | "development" => Ok(ApplicationProfile::Dev),
            "prod" | "production" => Ok(ApplicationProfile::Prod),
            s => Err(anyhow::anyhow!(
                "`{}` is not a valid application profile.\nValid options are: `dev`, `prod`.",
                s
            )),
        }
    }
}
