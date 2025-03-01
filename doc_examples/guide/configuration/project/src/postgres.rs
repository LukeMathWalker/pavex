use secrecy::Secret;

#[derive(serde::Deserialize, Debug, Clone)]
pub struct PostgresConfig {
    pub pool: PoolConfig,
    pub connection: ConnectionConfig,
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct ConnectionConfig {
    pub username: String,
    pub password: Secret<String>,
    pub host: String,
    pub database_name: String,
    pub require_ssl: bool,
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct PoolConfig {
    pub min_size: u32,
    pub max_size: u32,
}

pub fn register(bp: &mut Blueprint) {
    bp.config("postgres", t!(self::PostgresConfig));
}

use pavex::blueprint::Blueprint;
use pavex::t;
