use secrecy::Secret;

#[derive(serde::Deserialize, Debug, Clone)]
pub struct DatabaseConfig {
    pub username: String,
    pub password: Secret<String>,
    pub host: String,
    pub database_name: String,
    pub require_ssl: bool,
}
