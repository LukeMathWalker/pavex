use jsonwebtoken::{DecodingKey, EncodingKey};
use secrecy::{ExposeSecret, Secret};
use serde_aux::field_attributes::deserialize_number_from_string;
use sqlx::postgres::{PgConnectOptions, PgSslMode};
use std::net::{SocketAddr, TcpListener};

#[derive(serde::Deserialize)]
/// The top-level configuration, holding all the values required
/// to configure the entire application.
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub auth: AuthConfig,
}

#[derive(serde::Deserialize, Clone)]
/// Configuration for the HTTP server used to expose our API
/// to users.
pub struct ServerConfig {
    /// The port that the server must listen on.
    pub port: u16,
    /// The network interface that the server must be bound to.
    ///
    /// E.g. `0.0.0.0` for listening to incoming requests from
    /// all sources.
    pub ip: std::net::IpAddr,
}

impl ServerConfig {
    /// Bind a TCP listener according to the specified parameters.
    pub fn listener(&self) -> Result<TcpListener, std::io::Error> {
        let addr = SocketAddr::new(self.ip, self.port);
        TcpListener::bind(addr)
    }
}

#[derive(serde::Deserialize, Clone)]
pub struct DatabaseConfig {
    pub username: String,
    pub password: Secret<String>,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub host: String,
    pub database_name: String,
    pub require_ssl: bool,
}

impl DatabaseConfig {
    /// Return the database connection options.
    pub fn connection_options(&self) -> PgConnectOptions {
        let ssl_mode = if self.require_ssl {
            PgSslMode::Require
        } else {
            PgSslMode::Prefer
        };
        PgConnectOptions::new()
            .host(&self.host)
            .username(&self.username)
            .password(self.password.expose_secret())
            .port(self.port)
            .ssl_mode(ssl_mode)
            .database(&self.database_name)
    }

    /// Return a database connection pool.
    pub async fn get_pool(&self) -> Result<sqlx::PgPool, sqlx::Error> {
        let pool = sqlx::PgPool::connect_with(self.connection_options()).await?;
        Ok(pool)
    }
}

#[derive(serde::Deserialize, Clone)]
/// Configuration for the authentication system.
pub struct AuthConfig {
    /// The private key used to sign JWTs.
    pub eddsa_private_key_pem: Secret<String>,
    /// The public key used to verify the signature of JWTs.
    pub eddsa_public_key_pem: String,
}

impl AuthConfig {
    /// Return the private key to be used for JWT signing.
    pub fn encoding_key(&self) -> Result<EncodingKey, jsonwebtoken::errors::Error> {
        EncodingKey::from_ed_pem(self.eddsa_private_key_pem.expose_secret().as_bytes())
    }

    /// Return the public key to be used for verifying the signature of JWTs.
    pub fn decoding_key(&self) -> Result<DecodingKey, jsonwebtoken::errors::Error> {
        DecodingKey::from_ed_pem(self.eddsa_public_key_pem.as_bytes())
    }
}
