//! Extract data from the body of incoming requests.
//!
//! # Overview
//!
//! You can think of extractors for request bodies as a _layered_ system:
//!
//! 1. [`RawIncomingBody`] sits at the lowest level: everything else is built on top of it.
//!   It is the raw incoming body, the stream of bytes received by the server.
//!   You rarely want to work with this type directly.
//! 2. [`BufferedBody`] takes a [`RawIncomingBody`] and buffers it in memory.
//!   It also makes sure to enforce sane limits on the size of the body to avoid resource
//!   exhaustion attacks.
//!   You can use this type to extract the body of incoming requests as a bytes buffer.  
//! 3. Deserializers.  
//!    They take a [`BufferedBody`] as input and pass it over to a deserializer for a certain
//!    format (e.g. JSON) to extract the body as a structured type (e.g. a struct).
//!    Pavex provides [`JsonBody`] as an example of such a deserializer for JSON.  

pub use buffered_body::BufferedBody;
pub use json::JsonBody;
pub use limit::BodySizeLimit;
pub use raw_body::RawIncomingBody;

mod buffered_body;
pub mod errors;
mod json;
mod limit;
mod raw_body;
