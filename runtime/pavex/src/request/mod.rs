//! Process and extract data from incoming HTTP requests.
//!
//! # Guide
//!
//! Check out [the guide](https://pavex.dev/docs/guide/request_data/)
//! for a thorough introduction to request-based data extractors.
pub use request_head::RequestHead;

pub mod body;
pub mod path;
pub mod query;
mod request_head;
