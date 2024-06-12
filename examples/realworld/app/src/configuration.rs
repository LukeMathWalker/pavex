use jsonwebtoken::{DecodingKey, EncodingKey};
use pavex::cookie::ProcessorConfig;
use pavex::{blueprint::Blueprint, f, t};
use secrecy::{ExposeSecret, Secret};
use serde_aux::field_attributes::deserialize_number_from_string;
use sqlx::postgres::{PgConnectOptions, PgSslMode};

#[derive(serde::Deserialize, Debug, Clone)]
/// The configuration object holding all the values required
/// to configure the application.
pub struct ApplicationConfig {
    pub database: DatabaseConfig,
    pub auth: AuthConfig,
    #[serde(default)]
    pub cookie: ProcessorConfig,
}

impl ApplicationConfig {
    pub fn database_config(&self) -> &DatabaseConfig {
        &self.database
    }

    pub fn auth_config(&self) -> &AuthConfig {
        &self.auth
    }

    pub fn cookie_config(&self) -> ProcessorConfig {
        self.cookie.clone()
    }

    pub fn register(bp: &mut Blueprint) {
        bp.prebuilt(t!(self::ApplicationConfig));
        bp.transient(f!(self::ApplicationConfig::database_config));
        bp.transient(f!(self::ApplicationConfig::auth_config));
        bp.singleton(f!(self::ApplicationConfig::cookie_config));
        bp.singleton(f!(self::DatabaseConfig::get_pool));
        bp.singleton(f!(self::AuthConfig::decoding_key));
    }
}

#[derive(serde::Deserialize, Debug, Clone)]
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

#[derive(serde::Deserialize, Clone, Debug)]
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
