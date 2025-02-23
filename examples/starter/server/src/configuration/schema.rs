use crate::configuration::ApplicationProfile;
use anyhow::Context;
use app::configuration::AppConfig;
use figment::Figment;
use figment::providers::{Env, Format, Yaml};
use pavex::server::IncomingStream;
use std::net::SocketAddr;

#[derive(serde::Deserialize, Debug, Clone)]
/// The top-level configuration object, determining the schema
/// we expect to see in the configuration files stored under `api_server/configuration`.
///
/// # Location
///
/// It is defined in `server` since it bundles together the app configuration
/// ([`AppConfig`]) and the HTTP server configuration ([`ServerConfig`]).
/// The app configuration will be visible to constructors and other components,
/// while the HTTP server configuration will only be used inside the `main` entrypoint.
///
/// # Loading
///
/// Check out [`Config::load`]'s documentation for more details on how configuration
/// values are populated.
pub struct Config {
    pub server: ServerConfig,
    #[serde(flatten)]
    pub app: AppConfig,
}

impl Config {
    /// Retrieve the application configuration by merging multiple configuration sources.
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
    /// Your primary mechanism to specify the desired application profile is the `APP_PROFILE`
    /// environment variable.
    /// You can pass a `default_profile` value that will be used if the environment variable
    /// is not set.
    ///
    /// # Hierarchy
    ///
    /// The configuration sources are:
    ///
    /// 1. `base.yml` - The default configuration values, common to all profiles.
    /// 2. `<profile>.yml` - Configuration values specific to the desired profile.
    /// 3. Environment variables - Configuration values specific to the current environment.
    ///
    /// The configuration sources are listed in priority order, i.e.
    /// the last source in the list will override any previous source.
    ///
    /// For example, if the same configuration key is defined in both
    /// the YAML file and the environment, the value from the environment
    /// will be used.
    ///
    /// # Environment variables
    ///
    /// You can use environment variables to override every single configuration value.
    ///
    /// All config-related environment variables must be prefixed with `APP_`.
    /// The prefix tries to minimise the chance of a collision with unrelated environment
    /// variables that might be set on the host where the application is launched.
    ///
    /// After the `APP_` prefix, you must concatenate the names of the fields that must
    /// be traversed (starting from [`AppConfig`]) to reach the configuration value you want to
    /// override.
    ///
    /// E.g. `APP_SERVER__PORT` for [`ServerConfig::port`] since, to reach it, you go through:
    ///
    /// - the `server` field in [`AppConfig`]
    /// - the `port` field in [`ServerConfig`]
    ///
    /// We make an exception for [`AppConfig::app`].
    /// You can use `APP_` as prefix for its subfields rather than `APP_APP__*`.
    pub fn load(default_profile: Option<ApplicationProfile>) -> Result<Config, anyhow::Error> {
        let application_profile = ApplicationProfile::load(default_profile)
            .context("Failed to load the desired application profile")?;

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
            .merge(Env::prefixed("APP_").split("__"));

        let configuration: Config = figment
            .extract()
            .context("Failed to load hierarchical configuration")?;
        Ok(configuration)
    }
}

#[derive(serde::Deserialize, Debug, Clone)]
/// Configuration for the HTTP server used to expose our API
/// to users.
pub struct ServerConfig {
    /// The port that the server must listen on.
    ///
    /// Set the `APP_SERVER__PORT` environment variable to override its value.
    #[serde(deserialize_with = "serde_aux::field_attributes::deserialize_number_from_string")]
    pub port: u16,
    /// The network interface that the server must be bound to.
    ///
    /// E.g. `0.0.0.0` for listening to incoming requests from
    /// all sources.
    ///
    /// Set the `APP_SERVER__IP` environment variable to override its value.
    pub ip: std::net::IpAddr,
    /// The timeout for graceful shutdown of the server.
    ///
    /// E.g. `1 minute` for a 1 minute timeout.
    ///
    /// Set the `APP_SERVER__GRACEFUL_SHUTDOWN_TIMEOUT` environment variable to override its value.
    #[serde(with = "humantime_serde")]
    pub graceful_shutdown_timeout: std::time::Duration,
}

impl ServerConfig {
    /// Bind a TCP listener according to the specified parameters.
    pub async fn listener(&self) -> Result<IncomingStream, std::io::Error> {
        let addr = SocketAddr::new(self.ip, self.port);
        IncomingStream::bind(addr).await
    }
}
