use std::net::SocketAddr;

#[derive(Clone, Debug)]
pub struct ConnectionInfo {
    // pub(crate) local_addr: SocketAddr,
    pub(crate) peer_addr: SocketAddr,
}

impl ConnectionInfo {
    // pub fn local_addr(&self) -> SocketAddr {
    //     self.local_addr
    // }

    pub fn peer_addr(&self) -> SocketAddr {
        self.peer_addr
    }
}
