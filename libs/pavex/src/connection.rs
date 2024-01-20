use std::net::SocketAddr;

#[derive(Clone, Debug)]
pub struct ConnectionInfo {
    pub(crate) peer_addr: SocketAddr,
}

impl ConnectionInfo {
    pub fn peer_addr(&self) -> SocketAddr {
        self.peer_addr
    }
}
