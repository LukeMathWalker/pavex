use pavex::{Response, get};

use crate::{postgres::PostgresConfig, server::ServerConfig};

#[get(path = "/")]
pub fn handler(_db: &PostgresConfig, _server: &ServerConfig) -> Response {
    // Handler logic
    todo!()
}
