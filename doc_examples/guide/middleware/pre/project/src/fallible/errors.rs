use pavex::response::Response;
use super::AuthError;

pub fn auth_error_handler(_e: &AuthError) -> Response {
    Response::unauthorized()
}
