//! px:struct
use pavex::config;

#[config(key = "pool")] // px::ann:1
#[derive(serde::Deserialize, Debug, Clone)]
pub struct PoolConfig {
    pub max_n_connections: u32,
    pub min_n_connections: u32,
}
