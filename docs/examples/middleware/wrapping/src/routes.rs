use pavex::http::StatusCode;

#[pavex::get(path = "/")]
pub fn handler() -> StatusCode {
    StatusCode::OK
}
