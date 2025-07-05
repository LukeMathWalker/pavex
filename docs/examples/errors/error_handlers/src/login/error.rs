//! px:error_handler_def
use pavex::{http::StatusCode, methods};

#[derive(Debug)]
pub enum LoginError {
    InvalidCredentials,
    DatabaseError,
}

#[methods] // px::ann:1
impl LoginError {
    #[error_handler] // px::hl
    pub fn to_response(&self) -> StatusCode {
        match self {
            LoginError::InvalidCredentials => StatusCode::UNAUTHORIZED,
            LoginError::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
