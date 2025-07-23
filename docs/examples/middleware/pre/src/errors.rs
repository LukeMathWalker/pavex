use pavex::methods;
use pavex::response::Response;

#[derive(Debug)]
pub struct AuthError;

#[methods]
impl AuthError {
    #[error_handler]
    pub fn to_response(&self) -> Response {
        Response::unauthorized()
    }
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "No `Authorization` header found")
    }
}
