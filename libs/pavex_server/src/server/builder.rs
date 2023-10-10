use std::net::SocketAddr;

use crate::incoming::Incoming;
use crate::server::configuration::ServerConfiguration;
use crate::server::server::Server;

/// A builder for [`Server`]s.  
///
/// Check out [`Server::builder`] for more information.
pub struct ServerBuilder {
    config: ServerConfiguration,
    incoming: Vec<Incoming>,
}

impl ServerBuilder {
    pub(super) fn new() -> Self {
        Self {
            config: ServerConfiguration::default(),
            incoming: Vec::new(),
        }
    }

    /// Configure this [`ServerBuilder`] according to the values set in the [`ServerConfiguration`]
    /// passed as input parameter.
    /// It will overwrite any previous configuration set on this [`ServerBuilder`].
    ///
    /// If you want to retrieve the current configuration, use [`ServerBuilder::get_config`].
    pub fn set_config(mut self, config: ServerConfiguration) -> Self {
        self.config = config;
        self
    }

    /// Get a reference to the [`ServerConfiguration`] for this [`ServerBuilder`].
    ///
    /// If you want to overwrite the existing configuration, use [`ServerBuilder::set_config`].
    pub fn get_config(&self) -> &ServerConfiguration {
        &self.config
    }

    /// Bind the server to the given address: the server will accept incoming connections from this
    /// address when started.  
    /// Binding an address may fail (e.g. if the address is already in use), therefore this method
    /// may return an error.  
    ///
    /// # Note
    ///
    /// A [`Server`] can be bound to multiple addresses: just call this method multiple times with
    /// all the addresses you want to bind to.
    ///
    /// # Example: bind one address
    ///
    /// ```rust
    /// use std::net::SocketAddr;
    /// use pavex_server::Server;
    ///
    /// # async fn t() -> std::io::Result<()> {
    /// let addr1 = SocketAddr::from(([127, 0, 0, 1], 8080));
    ///
    /// let server = Server::builder()
    ///     .bind(addr1)
    ///     .await?
    ///     .build();
    ///  // [...]
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Example: bind multiple addresses
    ///
    /// ```rust
    /// use std::net::SocketAddr;
    /// use pavex_server::Server;
    ///
    /// # async fn t() -> std::io::Result<()> {
    /// let addr1 = SocketAddr::from(([127, 0, 0, 1], 8080));
    /// let addr2 = SocketAddr::from(([127, 0, 0, 1], 4000));
    ///
    /// let server = Server::builder()
    ///     .bind(addr1)
    ///     .await?
    ///     .bind(addr2)
    ///     .await?
    ///     .build();
    ///  // [...]
    /// # Ok(())
    /// # }
    pub async fn bind(mut self, addr: SocketAddr) -> std::io::Result<Self> {
        let incoming = Incoming::bind(addr).await?;
        self.incoming.push(incoming);
        Ok(self)
    }

    /// Build the [`Server`] from this [`ServerBuilder`].
    pub fn build(self) -> Server {
        Server::new(self.config, self.incoming)
    }
}
