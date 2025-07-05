use pavex::get;
use pavex::http::StatusCode;
use pavex::request::body::BufferedBody;

#[get(path = "/no_limit")]
pub fn no_limit_handler(_body: BufferedBody) -> StatusCode {
    StatusCode::OK
}
