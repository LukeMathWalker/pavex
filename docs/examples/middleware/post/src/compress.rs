//! px:compress
use crate::errors::CompressionError;
use pavex::post_process;
use pavex::response::Response;

#[post_process]
pub fn compress(response: Response) -> Result<Response, CompressionError> {
    let compressed = {
        // Try to compress the response
        Err(CompressionError) // px::skip
    }?;
    Ok(compressed)
}
