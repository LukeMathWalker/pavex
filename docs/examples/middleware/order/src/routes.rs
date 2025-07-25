use pavex::{get, http::StatusCode};

#[get(path = "/")]
pub fn get_index() -> StatusCode {
    StatusCode::OK
}
