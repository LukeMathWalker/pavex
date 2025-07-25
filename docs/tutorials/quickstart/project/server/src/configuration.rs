use pavex::config::ConfigProfile;

/// The configuration profile, i.e. a way to determine which set of
/// configuration values should be used.
///
/// Check out [the guide](https://pavex.dev/docs/guide/configuration/loading/#configuration-profile)
/// for more details on configuration profiles.
#[derive(ConfigProfile, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Profile {
    /// Local development profile.
    ///
    /// This is the profile you should use when running the application locally
    /// for exploratory testing.
    ///
    /// The corresponding configuration file is `dev.yml`.
    /// It's committed to the repository, and it's meant to contain exclusively
    /// non-sensitive configuration values.
    /// Sensitive configuration values for local development should instead go
    /// in the top-level `.env` file, which is instead **never** committed to version
    /// control.
    Dev,
    /// Production profile.
    ///
    /// This is the profile you should use when running the application in production—e.g.
    /// when deploying it to a staging or production environment, exposed to live traffic.
    ///
    /// The corresponding configuration file is `prod.yml`.
    /// It's committed to the repository, but it's meant to contain exclusively
    /// non-sensitive configuration values.
    /// Sensitive configuration values for production should be injected at runtime
    /// according to the specifics of your deployment target (e.g. via environment variables).
    Prod,
}
