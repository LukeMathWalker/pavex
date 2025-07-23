//! px:method_annotation
use pavex::methods;
use pavex::response::Response;

pub struct AuthError {
    _data: String, // px::skip
}

#[methods] // px::ann:1
impl AuthError {
    #[error_handler]
    pub fn to_response(&self) -> Response {
        Response::internal_server_error() // px::skip
    }
}
