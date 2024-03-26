pub use blueprint::blueprint;
pub use errors::compression_error_handler;
pub use mw::compress;
pub use routes::handler;

mod blueprint;
mod errors;
mod mw;
mod routes;

#[derive(Debug)]
pub struct CompressionError;

impl std::fmt::Display for CompressionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Failed to compress the outgoing response")
    }
}
