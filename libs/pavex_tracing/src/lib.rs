//! The official integration between [`tracing`] and the [Pavex] framework.
//!
//! [`tracing`]:https://docs.rs/tracing/0.1.40/tracing
//! [Pavex]: https://pavex.dev
mod root_span;

pub use root_span::RootSpan;
