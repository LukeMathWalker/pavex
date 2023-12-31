use pavex::http::StatusCode;
use pavex::request::body::BufferedBody;

pub fn handler(_body: BufferedBody) -> StatusCode {
    StatusCode::OK
}
