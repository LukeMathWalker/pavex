//! Extract data concerning the HTTP connection.
use std::net::SocketAddr;

/// Information relating to the current underlying HTTP connection.
///
/// It includes the [peer address](SocketAddr).
///
/// # Guide
///
/// Check out [the guide](https://pavex.dev/docs/guide/request_data/connection_info/)
/// for more details on `ConnectionInfo`.
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
    /// use pavex::response::Response;
    ///
    /// // The `ConnectionInfo` extractor can be used to access a peer's address within a handler.
    /// pub fn echo_ip(conn_info: &ConnectionInfo) -> Response {
    ///     let body = format!(
    ///         "The peer address for this connection is {}",
    ///         conn_info.peer_addr()
    ///     );
    ///     Response::ok().set_typed_body(body)
    /// }
    /// ```
    pub fn peer_addr(&self) -> SocketAddr {
        self.peer_addr
    }
}
