use pavex::http::StatusCode;
use pavex::request::body::BufferedBody;

pub fn upload(_body: BufferedBody) -> StatusCode {
    StatusCode::OK
}
