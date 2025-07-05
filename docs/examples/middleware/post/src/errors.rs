use pavex::methods;
use pavex::response::Response;

#[derive(Debug)]
pub struct CompressionError;

impl std::fmt::Display for CompressionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Failed to compress the outgoing response")
    }
}

#[methods]
impl CompressionError {
    #[error_handler]
    pub fn to_response(&self) -> Response {
        Response::internal_server_error()
    }
}
