use crate::incoming::Incoming;
use crate::server::configuration::ServerConfiguration;

pub struct Server {
    config: ServerConfiguration,
    incoming: Vec<Incoming>,
}

impl Server {
    pub(super) fn new(config: ServerConfiguration, incoming: Vec<Incoming>) -> Self {
        Self { config, incoming }
    }

    /// Initialize a new [`ServerConfiguration`] with its default configuration.
    pub fn builder() -> ServerConfiguration {
        ServerConfiguration::new()
    }
}
