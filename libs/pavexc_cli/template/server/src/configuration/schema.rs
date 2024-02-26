use crate::configuration::ApplicationProfile;
use anyhow::Context;
use app::configuration::AppConfig;
use figment::providers::{Env, Format, Yaml};
use figment::Figment;
use pavex::server::IncomingStream;
use std::net::SocketAddr;

#[derive(serde::Deserialize, Debug, Clone)]
/// The top-level configuration object, determining the schema
/// we expect to see in the configuration files stored under `api_server/configuration`.
///
/// It is defined in `api_server` since it bundles together the app configuration
/// ([`AppConfig`]) and the HTTP server configuration ([`ServerConfig`]).
/// The app configuration will be visible to constructors and other components,
/// while the HTTP server configuration won't.
pub struct Config {
    pub server: ServerConfig,
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
    pub port: u16,
    /// The network interface that the server must be bound to.
    ///
    /// E.g. `0.0.0.0` for listening to incoming requests from
    /// all sources.
    pub ip: std::net::IpAddr,
}

impl ServerConfig {
    /// Bind a TCP listener according to the specified parameters.
    pub async fn listener(&self) -> Result<IncomingStream, std::io::Error> {
        let addr = SocketAddr::new(self.ip, self.port);
        IncomingStream::bind(addr).await
    }
}
