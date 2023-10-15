use std::net::SocketAddr;

use socket2::Domain;
use tokio::net::{TcpListener, TcpStream};

/// A stream of incoming connections.  
/// [`IncomingStream::bind`] is the primary entrypoint for constructing a new [`IncomingStream`].
///
/// Incoming connections will be usually passed to a [`Server`](super::Server) instance to be handled.
/// Check out [`Server::bind`](super::Server::bind) or
/// [`Server::listen`](super::Server::listen) for more information.
///
pub struct IncomingStream {
    listener: TcpListener,
}

impl IncomingStream {
    /// Create a new [`IncomingStream`] by binding to a socket address.  
    /// The socket will be configured to be non-blocking and reuse the address.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::net::SocketAddr;
    /// use pavex::server::{IncomingStream, Server};
    ///
    /// # async fn t() -> std::io::Result<()> {
    /// let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    /// let incoming = IncomingStream::bind(addr).await?;
    /// # Ok(())
    /// # }
    /// ````
    ///
    /// # Custom configuration
    ///
    /// If you want to customize the options set on the socket, you can build your own
    /// [`TcpListener`](std::net::TcpListener) using [`socket2::Socket`] and then convert it
    /// into an [`IncomingStream`] via [`TryFrom::try_from`].
    ///
    /// ```rust
    /// use std::net::SocketAddr;
    /// use socket2::Domain;
    /// use pavex::server::{IncomingStream, Server};
    ///
    /// # async fn t() -> std::io::Result<()> {
    /// let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
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
    /// // We customize the backlog setting!
    /// socket.listen(2048_i32)?;
    ///
    /// let listener = std::net::TcpListener::from(socket);
    /// let incoming: IncomingStream = listener.try_into()?;
    /// # Ok(())
    /// # }
    /// ````
    pub async fn bind(addr: SocketAddr) -> std::io::Result<Self> {
        let socket = socket2::Socket::new(
            Domain::for_address(addr),
            socket2::Type::STREAM,
            Some(socket2::Protocol::TCP),
        )
        .expect("Failed to create a socket");

        socket.set_reuse_address(true)?;
        socket.set_nonblocking(true)?;
        socket.bind(&addr.into())?;
        socket.listen(1024_i32)?;

        let listener = std::net::TcpListener::from(socket);
        Ok(Self {
            listener: TcpListener::from_std(listener)?,
        })
    }

    /// Returns the address that this [`IncomingStream`] is bound to.
    pub fn local_addr(&self) -> std::io::Result<SocketAddr> {
        // The address we bound to may not be the same as the one we requested.
        // This happens, for example, when binding to port 0—this will cause the OS to pick a random
        // port for us which we won't know unless we call `local_addr` on the listener.
        self.listener.local_addr()
    }

    /// Accepts a new incoming connection from the underlying listener.
    ///
    /// This function will yield once a new TCP connection is established. When
    /// established, the corresponding [`TcpStream`] and the remote peer's
    /// address will be returned.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pavex::server::IncomingStream;
    /// use std::net::SocketAddr;
    ///
    /// # async fn t() -> std::io::Result<()> {
    /// let address = SocketAddr::from(([127, 0, 0, 1], 8080));
    /// let incoming = IncomingStream::bind(address).await?;
    ///
    /// match incoming.accept().await {
    ///     Ok((_socket, addr)) => println!("new client: {:?}", addr),
    ///     Err(e) => println!("couldn't get client: {:?}", e),
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn accept(&self) -> std::io::Result<(TcpStream, SocketAddr)> {
        self.listener.accept().await
    }
}

impl TryFrom<std::net::TcpListener> for IncomingStream {
    type Error = std::io::Error;

    fn try_from(v: std::net::TcpListener) -> std::io::Result<Self> {
        Ok(Self {
            listener: TcpListener::from_std(v)?,
        })
    }
}

impl From<TcpListener> for IncomingStream {
    fn from(v: TcpListener) -> Self {
        Self { listener: v }
    }
}
