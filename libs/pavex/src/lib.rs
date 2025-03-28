//! # Pavex - API reference
//!
//! Welcome to the API reference for Pavex!
//!
//! The API reference is fairly low-level.\
//! If you want a high-level overview of Pavex, check out the [documentation](https://pavex.dev/docs/)
//! on Pavex's website.\
//! You'll also find [an installation guide](https://pavex.dev/docs/getting_started/) as well as a
//! [quickstart tutorial](https://pavex.dev/docs/getting_started/quickstart/)
//! to get you up and running with the framework in no time.

pub use error::error_::Error;

pub mod blueprint;
#[cfg(feature = "config")]
pub mod config;
pub mod connection;
#[cfg(feature = "cookie")]
pub mod cookie;
pub mod error;
pub mod http;
pub mod kit;
pub mod middleware;
pub mod request;
pub mod response;
pub mod router;
pub mod serialization;
#[cfg(feature = "server")]
pub mod server;
pub mod telemetry;
pub mod unit;
pub mod time {
    //! Utilities to work with dates, timestamps and datetimes.
    //!
    //! It's a re-export of the [`jiff@0.2`](https://docs.rs/jiff/0.2) crate.
    pub use jiff::*;
}

pub use pavex_macros::constructor;
pub use pavex_macros::request_scoped;
pub use pavex_macros::singleton;
pub use pavex_macros::transient;
