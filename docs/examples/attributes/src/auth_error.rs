//! px:custom_id
use pavex::Response;
use pavex::methods;

pub struct AuthError {
    _data: String, // px::skip
}

#[methods]
impl AuthError {
    #[error_handler(id = "AUTH_ERROR_HANDLER")] // px::ann:1
    pub fn to_response(&self) -> Response {
        Response::internal_server_error() // px::skip
    }
}
