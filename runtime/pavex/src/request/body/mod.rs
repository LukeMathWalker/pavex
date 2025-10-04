//! Extract data from the body of incoming requests.
//!
//! Check the [relevant section of the guide](https://pavex.dev/docs/guide/request_data/body/)
//! for a thorough introduction to Pavex's body extractors.
pub use buffered_body::BufferedBody;
pub use json::JsonBody;
pub use limit::BodySizeLimit;
pub use raw_body::RawIncomingBody;
pub use url_encoded::UrlEncodedBody;

mod buffered_body;
pub mod errors;
mod json;
mod limit;
mod raw_body;
mod url_encoded;
