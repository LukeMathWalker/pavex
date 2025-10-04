//! The official integration between [`tracing`] and the [Pavex] framework.
//!
//! [`tracing`]:https://docs.rs/tracing/0.1.40/tracing
//! [Pavex]: https://pavex.dev
pub mod fields;
mod mw;
mod root_span;

pub use mw::{LOGGER, logger};
pub use root_span::RootSpan;
