#![deny(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]
//! Storage backends for [`pavex_session`](https://crates.io/crates/pavex_session),
//! implemented using the [`sqlx`](https://crates.io/crates/sqlx) crate.
//!
//! There is a dedicated feature flag for each supported database backend:
//!
//! - `postgres`: Support for PostgreSQL.
//! - `sqlite`: Support for SQLite.

#[cfg(feature = "postgres")]
#[cfg_attr(docsrs, doc(cfg(feature = "postgres")))]
pub mod postgres;

#[cfg(feature = "postgres")]
#[cfg_attr(docsrs, doc(cfg(feature = "postgres")))]
#[doc(inline)]
pub use postgres::PostgresSessionKit;

#[cfg(feature = "postgres")]
#[cfg_attr(docsrs, doc(cfg(feature = "postgres")))]
#[doc(inline)]
pub use postgres::PostgresSessionStore;

#[cfg(feature = "sqlite")]
#[cfg_attr(docsrs, doc(cfg(feature = "sqlite")))]
pub mod sqlite;

#[cfg(feature = "sqlite")]
#[cfg_attr(docsrs, doc(cfg(feature = "sqlite")))]
#[doc(inline)]
pub use sqlite::SqliteSessionKit;

#[cfg(feature = "sqlite")]
#[cfg_attr(docsrs, doc(cfg(feature = "sqlite")))]
#[doc(inline)]
pub use sqlite::SqliteSessionStore;
