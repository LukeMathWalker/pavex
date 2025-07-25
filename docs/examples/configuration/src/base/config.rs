//! px:derives
use pavex::config;
use redact::Secret;

#[config(key = "database")] // px::hl
#[derive(serde::Deserialize, Debug, Clone)] // px::ann:1
pub struct DatabaseConfig {
    pub username: String,
    pub password: Secret<String>,
    pub host: String,
    pub database_name: String,
    pub require_ssl: bool,
}
