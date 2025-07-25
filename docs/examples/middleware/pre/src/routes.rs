use pavex::{get, http::StatusCode};

#[get(path = "/")]
pub fn handler() -> StatusCode {
    StatusCode::OK
}
