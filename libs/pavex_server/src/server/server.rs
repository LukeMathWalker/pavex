use crate::server::builder::ServerBuilder;

pub struct Server {}

impl Server {
    pub fn builder() -> ServerBuilder {
        ServerBuilder::new()
    }
}
