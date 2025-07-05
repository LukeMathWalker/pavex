//! Refer to Pavex's [configuration guide](https://pavex.dev/docs/guide/configuration) for more details
//! on how to manage configuration values.
use pavex::config;
use pavex::server::IncomingStream;

#[derive(serde::Deserialize, Debug, Clone)]
/// Configuration for the HTTP server used to expose our API
/// to users.
#[config(key = "server", include_if_unused)]
pub struct ServerConfig {
    /// The port that the server must listen on.
    ///
    /// Set the `PX_SERVER__PORT` environment variable to override its value.
    #[serde(deserialize_with = "serde_aux::field_attributes::deserialize_number_from_string")]
    pub port: u16,
    /// The network interface that the server must be bound to.
    ///
    /// E.g. `0.0.0.0` for listening to incoming requests from
    /// all sources.
    ///
    /// Set the `PX_SERVER__IP` environment variable to override its value.
    pub ip: std::net::IpAddr,
    /// The timeout for graceful shutdown of the server.
    ///
    /// E.g. `1 minute` for a 1 minute timeout.
    ///
    /// Set the `PX_SERVER__GRACEFUL_SHUTDOWN_TIMEOUT` environment variable to override its value.
    #[serde(deserialize_with = "deserialize_shutdown")]
    pub graceful_shutdown_timeout: std::time::Duration,
}

fn deserialize_shutdown<'de, D>(deserializer: D) -> Result<std::time::Duration, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize as _;

    let duration = pavex::time::SignedDuration::deserialize(deserializer)?;
    if duration.is_negative() {
        Err(serde::de::Error::custom(
            "graceful shutdown timeout must be positive",
        ))
    } else {
        duration.try_into().map_err(serde::de::Error::custom)
    }
}

impl ServerConfig {
    /// Bind a TCP listener according to the specified parameters.
    pub async fn listener(&self) -> Result<IncomingStream, std::io::Error> {
        let addr = std::net::SocketAddr::new(self.ip, self.port);
        IncomingStream::bind(addr).await
    }
}
