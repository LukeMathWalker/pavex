use pavex::get;
use pavex::http::StatusCode;

#[get(path = "/")]
pub fn handler() -> StatusCode {
    StatusCode::OK
}
