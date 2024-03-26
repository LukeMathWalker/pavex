use pavex::response::Response;
use super::CompressionError;

pub fn compression_error_handler(_e: &CompressionError) -> Response {
    Response::internal_server_error()
}
