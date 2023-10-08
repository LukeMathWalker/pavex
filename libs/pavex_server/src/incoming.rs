use std::net::SocketAddr;

use tokio::net::TcpListener;

/// A stream of incoming connections.
pub struct Incoming {
    addr: SocketAddr,
    listener: TcpListener,
}

impl Incoming {
    // TODO: should we use a custom error type to capture which address failed to bind?
    /// Creates a new [`Incoming`] by binding to a socket address.
    pub fn bind(addr: SocketAddr) -> std::io::Result<Self> {
        let listener = TcpListener::bind(addr)?;
        // The address we bound to may not be the same as the one we requested.
        // This happens, for example, when binding to port 0—this will cause the OS to pick a random
        // port for us which we won't know unless we call `local_addr` on the listener.
        let addr = listener.local_addr()?;
        Ok(Self { addr, listener })
    }
}
