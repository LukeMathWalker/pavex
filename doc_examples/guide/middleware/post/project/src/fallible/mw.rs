use pavex::response::Response;
use super::CompressionError;

pub fn compress(response: Response) -> Result<Response, CompressionError>
{
    let compressed = {
        // Try to compress the response
        Err(CompressionError)
    }?;
    Ok(compressed)
}
