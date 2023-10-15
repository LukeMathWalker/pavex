use std::net::SocketAddr;

use tokio::net::{TcpListener, TcpStream};

/// A stream of incoming connections.
pub struct IncomingStream {
    addr: SocketAddr,
    listener: TcpListener,
}

impl IncomingStream {
    // TODO: should we use a custom error type to capture which address failed to bind?
    /// Creates a new [`IncomingStream`] by binding to a socket address.
    pub async fn bind(addr: SocketAddr) -> std::io::Result<Self> {
        let listener = TcpListener::bind(addr).await?;
        // The address we bound to may not be the same as the one we requested.
        // This happens, for example, when binding to port 0—this will cause the OS to pick a random
        // port for us which we won't know unless we call `local_addr` on the listener.
        let addr = listener.local_addr()?;
        Ok(Self { addr, listener })
    }

    /// Returns the address that this [`IncomingStream`] is bound to.
    pub fn local_addr(&self) -> &SocketAddr {
        &self.addr
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
    /// }
    /// ```
    pub async fn accept(&self) -> std::io::Result<(TcpStream, SocketAddr)> {
        self.listener.accept().await
    }
}
