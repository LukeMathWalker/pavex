use pavex::http::StatusCode;

pub fn greet() -> Result<StatusCode, GreetError> {
    StatusCode::OK
}

pub enum GreetError {
    DatabaseError,
    InvalidName,
}
