use pavex::get;
use pavex::http::StatusCode;
use pavex::request::body::BufferedBody;

#[get(path = "/custom_limit")]
pub fn custom_limit_handler(_body: BufferedBody) -> StatusCode {
    StatusCode::OK
}
