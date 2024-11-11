//! Storage backends for `pavex_session`, implemented using the `sqlx` crate.
//!
//! There is a dedicated feature flag for each supported database backend:
//!
//! - `postgres`: Support for PostgreSQL.

#[cfg(feature = "postgres")]
mod postgres;
#[cfg(feature = "postgres")]
pub use postgres::PostgresSessionStore;
