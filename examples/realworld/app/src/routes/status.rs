use pavex::get;
use pavex::http::StatusCode;

#[get(path = "/ping")]
pub fn ping() -> StatusCode {
    StatusCode::OK
}
