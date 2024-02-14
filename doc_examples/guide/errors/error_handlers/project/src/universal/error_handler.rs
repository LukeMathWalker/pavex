use pavex::response::Response;

pub async fn login_error2response(e: &pavex::Error) -> Response {
    Response::unauthorized().set_typed_body(e.to_string())
}

#[derive(Debug)]
pub enum LoginError {
    InvalidCredentials,
    DatabaseError,
}
