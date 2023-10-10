use crate::incoming::Incoming;
use crate::server::configuration::ServerConfiguration;
use crate::ServerBuilder;

pub struct Server {
    config: ServerConfiguration,
    incoming: Vec<Incoming>,
}

impl Server {
    pub(super) fn new(config: ServerConfiguration, incoming: Vec<Incoming>) -> Self {
        Self { config, incoming }
    }

    /// Configure a [`Server`] using a [`ServerBuilder`].
    pub fn builder() -> ServerBuilder {
        ServerBuilder::new()
    }
}
