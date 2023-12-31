use pavex::http::StatusCode;
use pavex::request::body::BufferedBody;

pub fn handler(body: &BufferedBody) -> StatusCode {
    format!("The incoming request contains {} bytes", body.bytes.len());
    StatusCode::OK
}
