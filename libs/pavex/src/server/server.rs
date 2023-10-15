use std::future::Future;
use std::net::SocketAddr;

use crate::server::configuration::ServerConfiguration;
use crate::server::server_handle::ServerHandle;

use super::IncomingStream;

/// A builder for [`ServerHandle`]s.
///
/// Check out [`ServerHandle::builder`] for more information.
#[must_use = "You must call `serve` on a `Server` to start listening for incoming connections"]
pub struct Server {
    config: ServerConfiguration,
    incoming: Vec<IncomingStream>,
}

impl Default for Server {
    fn default() -> Self {
        Self::new()
    }
}

impl Server {
    /// Create a new [`Server`] with default configuration.
    pub fn new() -> Self {
        Self {
            config: ServerConfiguration::default(),
            incoming: Vec::new(),
        }
    }

    /// Configure this [`Server`] according to the values set in the [`ServerConfiguration`]
    /// passed as input parameter.
    /// It will overwrite any previous configuration set on this [`Server`].
    ///
    /// If you want to retrieve the current configuration, use [`Server::get_config`].
    pub fn set_config(mut self, config: ServerConfiguration) -> Self {
        self.config = config;
        self
    }

    /// Get a reference to the [`ServerConfiguration`] for this [`Server`].
    ///
    /// If you want to overwrite the existing configuration, use [`Server::set_config`].
    pub fn get_config(&self) -> &ServerConfiguration {
        &self.config
    }

    /// Bind the server to the given address: the server will accept incoming connections from this
    /// address when started.  
    /// Binding an address may fail (e.g. if the address is already in use), therefore this method
    /// may return an error.  
    ///
    /// # Related
    ///
    /// Check out [`Server::listen`] for an alternative binding mechanism as well as a
    /// discussion of the pros and cons of [`Server::bind`] vs [`Server::listen`].
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
    /// use pavex::server::Server;
    ///
    /// # async fn t() -> std::io::Result<()> {
    /// let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    ///
    /// Server::new()
    ///     .bind(addr)
    ///     .await?
    ///     # ;
    ///     // [...]
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Example: bind multiple addresses
    ///
    /// ```rust
    /// use std::net::SocketAddr;
    /// use pavex::server::Server;
    ///
    /// # async fn t() -> std::io::Result<()> {
    /// let addr1 = SocketAddr::from(([127, 0, 0, 1], 8080));
    /// let addr2 = SocketAddr::from(([127, 0, 0, 1], 4000));
    ///
    /// Server::new()
    ///     .bind(addr1)
    ///     .await?
    ///     .bind(addr2)
    ///     .await?
    ///     # ;
    ///     // [...]
    /// # Ok(())
    /// # }
    /// ````
    pub async fn bind(mut self, addr: SocketAddr) -> std::io::Result<Self> {
        let incoming = IncomingStream::bind(addr).await?;
        self.incoming.push(incoming);
        Ok(self)
    }

    /// Ask the server to process incoming connections from the provided [`IncomingStream`].  
    ///
    /// # [`Server::listen`] vs [`Server::bind`]
    ///
    /// [`Server::bind`] only requires you to specify the address you want to listen at. The
    /// socket configuration is handled by the [`Server`], with a set of reasonable default
    /// parameters. You have no access to the [`IncomingStream`] that gets bound to the address
    /// you specified.
    ///
    /// [`Server::listen`], instead, expects an [`IncomingStream`].  
    /// You are free to configure the socket as you see please and the [`Server`] will just
    /// poll it for incoming connections.  
    /// It also allows you to interact with the bound [`IncomingStream`] directly
    ///
    /// # Example: bind to a random port
    ///
    /// ```rust
    /// use std::net::SocketAddr;
    /// use pavex::server::{IncomingStream, Server};
    ///
    /// # async fn t() -> std::io::Result<()> {
    /// // `0` is a special port: it tells the OS to assign us
    /// // a random **unused** port
    /// let addr = SocketAddr::from(([127, 0, 0, 1], 0));
    /// let incoming = IncomingStream::bind(addr).await?;
    /// // We can then retrieve the actual port we were assigned
    /// // by the OS.
    /// let addr = incoming.local_addr()?.to_owned();
    ///
    /// Server::new()
    ///     .listen(incoming);
    ///     # ;
    ///     // [...]
    /// # Ok(())
    /// # }
    /// ````
    ///
    /// # Example: set a custom socket backlog
    ///
    /// ```rust
    /// use std::net::SocketAddr;
    /// use socket2::Domain;
    /// use pavex::server::{IncomingStream, Server};
    ///
    /// # async fn t() -> std::io::Result<()> {
    /// // `0` is a special port: it tells the OS to assign us
    /// // a random **unused** port
    /// let addr = SocketAddr::from(([127, 0, 0, 1], 0));
    ///
    /// let socket = socket2::Socket::new(
    ///    Domain::for_address(addr),
    ///    socket2::Type::STREAM,
    ///    Some(socket2::Protocol::TCP),
    /// )
    /// .expect("Failed to create a socket");
    /// socket.set_reuse_address(true)?;
    /// socket.set_nonblocking(true)?;
    /// socket.bind(&addr.into())?;
    /// // The custom backlog!
    /// socket.listen(2048_i32)?;
    ///
    /// let listener = std::net::TcpListener::from(socket);
    /// Server::new()
    ///     .listen(listener.try_into()?)
    ///     # ;
    ///     // [...]
    /// # Ok(())
    /// # }
    /// ````
    ///
    /// # Note
    ///
    /// A [`Server`] can listen to multiple streams of incoming connections: just call this method
    /// multiple times!
    pub fn listen(mut self, incoming: IncomingStream) -> Self {
        self.incoming.push(incoming);
        self
    }

    /// Start listening for incoming connections.
    ///
    /// You must specify:
    ///
    /// - a handler function, which will be called for each incoming request;
    /// - the application state, the set of singleton components that will be available to
    ///   your handler function.
    ///
    /// Both the handler function and the application state are usually code-generated by Pavex
    /// starting from your [`Blueprint`](crate::blueprint::Blueprint).
    pub fn serve<HandlerFuture, ApplicationState>(
        self,
        handler: fn(http::Request<hyper::body::Incoming>, ApplicationState) -> HandlerFuture,
        application_state: ApplicationState,
    ) -> ServerHandle
    where
        HandlerFuture: Future<Output = crate::response::Response> + 'static,
        ApplicationState: Clone + Send + Sync + 'static,
    {
        ServerHandle::new(self.config, self.incoming, handler, application_state)
    }
}
