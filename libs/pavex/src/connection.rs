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
    ///
    /// # Example
    ///
    /// ```rust
    /// use pavex::connection::ConnectionInfo;
    ///
    /// // The `ConnectionInfo` extractor can be used to access a peer's address within a handler.
    /// pub fn my_ip(conn_info: &ConnectionInfo) -> String {
    ///     format!("Your IP is {}", conn_info.peer_addr())
    /// }
    /// ```
    pub fn peer_addr(&self) -> SocketAddr {
        self.peer_addr
    }
}
