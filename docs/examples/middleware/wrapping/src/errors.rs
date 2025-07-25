use pavex::Response;
use tokio::time::error::Elapsed;

#[pavex::error_handler]
pub fn timeout_error_handler(_e: &Elapsed) -> Response {
    Response::internal_server_error()
}
