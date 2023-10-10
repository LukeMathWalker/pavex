use std::net::SocketAddr;

use crate::incoming::Incoming;
use crate::server::configuration::ServerConfiguration;
use crate::server::server::Server;

pub struct ServerBuilder {
    config: ServerConfiguration,
    incoming: Vec<Incoming>,
}

impl ServerBuilder {
    /// Set the [`ServerConfiguration`] for this [`ServerBuilder`].
    pub fn with_config(mut self, config: ServerConfiguration) -> Self {
        self.config = config;
        self
    }

    /// Bind the server to the given address.
    pub async fn bind(mut self, addr: SocketAddr) -> std::io::Result<Self> {
        let incoming = Incoming::bind(addr).await?;
        self.incoming.push(incoming);
        Ok(self)
    }

    /// Bind the server to the given address.
    pub async fn bind_with_config(
        mut self,
        addr: SocketAddr,
        config: ServerConfiguration,
    ) -> std::io::Result<Self> {
        let incoming = Incoming::bind(addr).await?;
        self.incoming.push(incoming);
        self.config = config;
        Ok(self)
    }

    /// Build the [`Server`] from this [`ServerBuilder`].
    pub fn build(self) -> Server {
        Server::new(self.config, self.incoming)
    }
}
