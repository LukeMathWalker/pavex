use pavex::http::StatusCode;
use pavex::post;
use pavex::request::body::BufferedBody;

#[post(path = "/upload")]
pub fn upload(_body: BufferedBody) -> StatusCode {
    StatusCode::OK
}
