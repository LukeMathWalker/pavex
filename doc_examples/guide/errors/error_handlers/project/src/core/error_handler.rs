use pavex::http::StatusCode;

pub async fn login_error2response(e: &LoginError) -> StatusCode /* (1)! */ {
    match e {
        LoginError::InvalidCredentials => StatusCode::UNAUTHORIZED,
        LoginError::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

#[derive(Debug)]
pub enum LoginError {
    InvalidCredentials,
    DatabaseError,
}
