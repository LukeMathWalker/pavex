//! # Pavex - API reference
//!
//! Welcome to the API reference for Pavex!
//!
//! The API reference is fairly low-level.  
//! If you want a high-level overview of Pavex, check out the [documentation](https://pavex.dev/docs/)
//! on Pavex's website.  
//! You'll also find [an installation guide](https://pavex.dev/docs/getting_started/) as well as a
//! [quickstart tutorial](https://pavex.dev/docs/getting_started/quickstart/)
//! to get you up and running with the framework in no time.

// Re-export the dependencies that we use in the generated application code.
pub use error::Error;

pub mod blueprint;
#[cfg(feature = "cookie")]
pub mod cookie;
mod error;
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
    //! It's a re-export of the [`time@0.3`](https://docs.rs/time/0.3) crate.
    pub use time::*;
}
