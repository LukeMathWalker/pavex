//! px:body
use pavex::http::StatusCode;
use pavex::post;
use pavex::request::body::RawIncomingBody;

#[post(path = "/body")]
pub fn raw_body(body: RawIncomingBody) -> StatusCode {
    StatusCode::OK // px::skip
}
