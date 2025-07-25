//! px:extraction
use pavex::http::StatusCode;
use pavex::post;
use pavex::request::body::BufferedBody;

#[post(path = "/buffered")]
pub fn buffered(body: &BufferedBody) -> StatusCode {
    println!("The incoming request contains {} bytes", body.bytes.len());
    StatusCode::OK // px::skip
}
