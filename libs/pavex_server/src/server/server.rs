use crate::server::configuration::ServerConfiguration;

pub struct Server {}

impl Server {
    /// Initialize a new [`ServerConfiguration`] with its default configuration.
    pub fn builder() -> ServerConfiguration {
        ServerConfiguration::new()
    }
}
