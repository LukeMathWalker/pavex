use pavex::{blueprint::Blueprint, t};

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ServerConfig {
    pub port: u16,
    pub host: String,
}

pub fn register(bp: &mut Blueprint) {
    bp.config("server", t!(self::ServerConfig));
}
