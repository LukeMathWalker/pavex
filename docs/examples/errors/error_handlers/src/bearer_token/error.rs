//! px:bearer_error
use pavex::methods;
use pavex::response::Response;

#[derive(Debug, Clone, thiserror::Error)]
pub enum BearerExtractionError {
    #[error("The request didn't set the `Authorization` header")]
    MissingAuthorizationHeader,
    #[error("The `Authorization` header is malformed")]
    MalformedHeader,
}

#[rustfmt::skip] // px::skip
#[methods]
impl BearerExtractionError {
    #[error_handler] // px::hl
    pub fn to_response(&self) -> Response {
        use BearerExtractionError::*;

        match self {
            MissingAuthorizationHeader => {
                Response::unauthorized()
                    .set_typed_body("Missing `Authorization` header")
            }
            MalformedHeader => {
                Response::bad_request()
                    .set_typed_body("Malformed `Authorization` header")
            }
        }
    }
}
