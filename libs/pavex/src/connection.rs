//! Extract data concerning the HTTP connection.
use std::net::SocketAddr;

/// Information relating to the current undelying HTTP connection.
///
/// It includes the [peer address](SocketAddr).
#[derive(Clone, Debug)]
pub struct ConnectionInfo {
    pub(crate) peer_addr: SocketAddr,
}

impl ConnectionInfo {
    /// Returns the peer address.
    pub fn peer_addr(&self) -> SocketAddr {
        self.peer_addr
    }
}
